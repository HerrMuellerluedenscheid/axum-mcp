mod counter;

use crate::counter::Counter;
use axum::extract::State;
use axum::{Router, routing::get};
use rmcp::transport::{SseServer, sse_server::SseServerConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
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
    let addr = BIND_ADDRESS.parse::<SocketAddr>().unwrap();

    let sse_config = SseServerConfig {
        bind: addr.into(),
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: Some(Duration::from_secs(15)),
    };
    let app_state = Arc::new(AppState {
        data: "XXX".to_string(),
    });

    let (sse_server, sse_router) = SseServer::new(sse_config);

    sse_server.with_service(Counter::new);

    let router = Router::new()
        .route("/hello", get(hello_world))
        .nest_service("/mcp", sse_router)
        .with_state(app_state);

    let app = router.into_make_service();
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
