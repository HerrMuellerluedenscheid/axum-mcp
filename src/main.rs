mod counter;

use crate::counter::Counter;
use axum::extract::State;
use axum::{Router, routing::get};
use rmcp::transport::StreamableHttpServer;
use rmcp::transport::common::axum::DEFAULT_AUTO_PING_INTERVAL;
use rmcp::transport::streamable_http_server::axum::{
    App, StreamableHttpServerConfig, delete_handler, get_handler, post_handler,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

const BIND_ADDRESS: &'static str = "127.0.0.1:3000";

#[derive(Clone)]
struct AppState {
    data: String,
}

async fn hello_world(state: State<Arc<AppState>>) -> String {
    "Hello world! ".to_string() + &state.data
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting MCP server");

    let addr = BIND_ADDRESS.parse::<SocketAddr>().unwrap();

    let config = StreamableHttpServerConfig {
        bind: addr.into(),
        ct: CancellationToken::new(),
        sse_keep_alive: Some(Duration::from_secs(15)),
        path: "/".to_string(),
    };
    let app_state = Arc::new(AppState {
        data: "XXX".to_string(),
    });

    let app_state_b = Arc::new(AppState {
        data: "XXX".to_string(),
    });

    let (app, transport_rx) = App::new(config.sse_keep_alive.unwrap_or(DEFAULT_AUTO_PING_INTERVAL));

    let sse_router = Router::new()
        .route(
            &config.path,
            get(get_handler).post(post_handler).delete(delete_handler),
        )
        .with_state(app);

    let sse_server = StreamableHttpServer {
        transport_rx,
        config,
    };

    sse_server.with_service(Counter::new);
    let router = Router::new()
        .route("/hello", get(hello_world))
        .nest("/mcp", sse_router)
        .with_state(app_state)
        .with_state(app_state_b);

    let app = router.into_make_service();
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
