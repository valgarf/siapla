use axum::{
    Extension, Router, middleware,
    routing::{MethodFilter, get, on},
};
use juniper::DefaultScalarValue;
use juniper_axum::{extract::JuniperRequest, graphiql, playground, response::JuniperResponse};
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
    JuniperResponse(req.execute(&schema, &context).await)
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

    let local_set = tokio::task::LocalSet::new();
    local_set.spawn_local(recalculate_loop());
    // tokio::spawn(recalculate_loop());

    let cors = CorsLayer::new()
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
        .allow_origin(tower_http::cors::Any);

    let app = Router::new()
        .route("/graphql", on(MethodFilter::GET.or(MethodFilter::POST), graphql))
        // .route(
        //     "/subscriptions",
        //     get(ws::<Arc<Schema>>(ConnectionConfig::new(()))),
        // )
        .route("/graphiql", get(graphiql("/graphql", "/subscriptions")))
        .route("/playground", get(playground("/graphql", "/subscriptions")))
        .layer(
            ServiceBuilder::new()
                .layer(cors)
                .layer(Extension(Arc::new(siapla::gql::schema())))
                // .layer(Extension(Context::new())),
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
