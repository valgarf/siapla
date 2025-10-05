use axum::{
    Extension, Router,
    extract::WebSocketUpgrade,
    middleware,
    response::Response,
    routing::{MethodFilter, get, on},
};
use juniper::DefaultScalarValue;
use juniper_axum::{
    extract::JuniperRequest, graphiql, playground, response::JuniperResponse, subscriptions,
};
use juniper_graphql_ws::ConnectionConfig;
use siapla::app_state::AppState;
use siapla::{
    gql::{
        Schema,
        context::{Context, add_context},
    },
    scheduling::recalculate_loop,
};
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
        .layer(
            ServiceBuilder::new()
                .layer(cors)
                .layer(Extension(Arc::new(siapla::gql::schema())))
                .layer(Extension(Arc::clone(&app_state)))
                .layer(middleware::from_fn(add_context)),
        );
    // .route("/", get(homepage))

    let addr = SocketAddr::from(([127, 0, 0, 1], 8880));
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
