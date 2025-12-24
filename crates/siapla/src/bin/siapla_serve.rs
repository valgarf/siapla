use axum::body::Body;
use axum::http::{Response as HttpResponse, StatusCode, header::CONTENT_TYPE};
use axum::response::IntoResponse;
use axum::{
    Extension, Router,
    extract::WebSocketUpgrade,
    middleware,
    response::Response,
    routing::{MethodFilter, get, on},
};
use clap::Parser;
use include_dir::{Dir, include_dir};
use juniper::DefaultScalarValue;
use juniper_axum::{
    extract::JuniperRequest, graphiql, playground, response::JuniperResponse, subscriptions,
};
use juniper_graphql_ws::ConnectionConfig;
use siapla::app_state::AppState;
use siapla::gql::context::set_global_database_url;
use siapla::{
    gql::{
        Schema,
        context::{Context, add_context},
    },
    scheduling::recalculate_loop,
};
use siapla_migration::MigratorTrait as _;
use sqlx::migrate::MigrateDatabase as _;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;
// use juniper_graphql_ws::ConnectionConfig;
// use tokio_stream::wrappers::IntervalStream;

#[axum::debug_handler]
pub async fn graphql(
    Extension(schema): Extension<Arc<Schema>>,
    Extension(context): Extension<Arc<Context>>,
    JuniperRequest(req): JuniperRequest<DefaultScalarValue>,
) -> JuniperResponse<DefaultScalarValue> {
    let gql_res = req.execute(&schema, &context).await;
    if !gql_res.is_ok() {
        context.failed().await;
    }
    let jun_res = JuniperResponse(gql_res);
    jun_res
}

async fn custom_subscriptions(
    Extension(schema): Extension<Arc<Schema>>,
    Extension(app_state): Extension<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.protocols(["graphql-transport-ws", "graphql-ws"])
        // .max_frame_size(1024)
        // .max_message_size(1024)
        // .max_write_buffer_size(100)
        .on_upgrade(move |socket| {
            subscriptions::serve_ws(
                socket,
                schema,
                ConnectionConfig::new(
                    Arc::try_unwrap(Context::new(app_state))
                        .expect("Arc has just been created, must be able to unwrap it."),
                )
                .with_max_in_flight_operations(10),
            )
        })
}

// Embed the bundled frontend directory at compile time
// include the bundled_frontend directory that lives in the same crate
static BUNDLED_FRONTEND_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/bundled_frontend");

#[derive(Parser, Debug)]
#[command(name = "siapla-serve")]
struct Args {
    /// Database URL to use (overrides DATABASE_URL env var)
    #[arg(long, default_value = "sqlite:./run-data/test.sqlite")]
    database_url: String,
    /// Bind address e.g. 127.0.0.1:8880
    #[arg(long, default_value = "0.0.0.0:80")]
    bind: String,
}

fn file_response_from_dir(mut path: String) -> Response {
    // normalize the path, try index.html for directories
    if path == "" || path.ends_with('/') {
        path = format!("{}/index.html", path.trim_end_matches('/'))
    }
    match BUNDLED_FRONTEND_DIR.get_file(path.trim_start_matches('/')) {
        Some(file) => {
            let ct = mime_guess::from_path(file.path()).first_or_text_plain();
            HttpResponse::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, ct.as_ref())
                .body(Body::from(file.contents().to_vec()))
                .unwrap()
        }
        None => HttpResponse::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap(),
    }
}

async fn serve_frontend(path: Option<String>) -> impl IntoResponse {
    let p = path.unwrap_or_else(|| "index.html".into());
    file_response_from_dir(p)
}

async fn init_db(db_url: &str) -> anyhow::Result<()> {
    if db_url.starts_with("sqlite:") {
        if !sqlx::Sqlite::database_exists(&db_url).await? {
            sqlx::Sqlite::create_database(&db_url).await?;
        }
    }
    // Initialize database connection
    let connect_opts = sea_orm::ConnectOptions::new(db_url).to_owned();
    let db = sea_orm::Database::connect(connect_opts).await?;

    // Upgrade to latest state
    siapla_migration::Migrator::up(&db, None).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_span_events(
            tracing_subscriber::fmt::format::FmtSpan::CLOSE
                | tracing_subscriber::fmt::format::FmtSpan::NEW,
        )
        .compact()
        .with_env_filter(EnvFilter::try_new("debug").unwrap())
        .init();

    // parse cli args and set global database url
    let args = Args::parse();
    init_db(&args.database_url).await?;
    set_global_database_url(args.database_url);

    let (app_state, manual_rx) = AppState::new();

    let local_set = tokio::task::LocalSet::new();
    // spawn scheduling loop with access to app_state
    let app_state_for_loop = Arc::clone(&app_state);
    local_set.spawn_local(async move { recalculate_loop(app_state_for_loop, manual_rx).await });
    // tokio::spawn(recalculate_loop());

    let cors = CorsLayer::new()
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
        .allow_origin(tower_http::cors::Any);

    let app = Router::new()
        .route("/graphql", on(MethodFilter::GET.or(MethodFilter::POST), graphql))
        .route("/subscriptions", get(custom_subscriptions))
        .route("/graphiql", get(graphiql("/graphql", "/subscriptions")))
        .route("/playground", get(playground("/graphql", "/subscriptions")))
        // serve bundled frontend at root
        .route("/", get(|| async { file_response_from_dir("index.html".to_string()) }))
        .route(
            "/{*path}",
            get(|axum::extract::Path(path): axum::extract::Path<String>| async move {
                serve_frontend(Some(path)).await
            }),
        )
        .layer(
            ServiceBuilder::new()
                .layer(cors)
                .layer(Extension(Arc::new(siapla::gql::schema())))
                .layer(Extension(Arc::clone(&app_state)))
                .layer(middleware::from_fn(add_context)),
        );
    // .route("/", get(homepage))

    let addr: SocketAddr = args.bind.parse().unwrap_or(SocketAddr::from(([0, 0, 0, 0], 80)));
    let listener =
        TcpListener::bind(addr).await.unwrap_or_else(|e| panic!("failed to listen on {addr}: {e}"));
    info!("listening on {addr}");
    let jh = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .unwrap_or_else(|e| panic!("failed to run `axum::serve`: {e}"));
    });
    local_set.spawn_local(async move { jh.await });
    local_set.await;
    Ok(())
}
