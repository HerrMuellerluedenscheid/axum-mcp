mod counter;

use crate::counter::Counter;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::Event;
use axum::response::{Response, Sse};
use axum::{Json, Router, routing::get};
use futures::Stream;
use rmcp::model::ClientJsonRpcMessage;
use rmcp::transport::StreamableHttpServer;
use rmcp::transport::common::axum::DEFAULT_AUTO_PING_INTERVAL;
use rmcp::transport::streamable_http_server::axum::{
    App, StreamableHttpServerConfig, delete_handler, get_handler, post_handler,
};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

const BIND_ADDRESS: &'static str = "127.0.0.1:3000";

#[derive(Clone)]
struct AppState {
    data: String,
}

struct CombinedState {
    a: State<AppState>,
    app_state: State<App>,
}

async fn hello_world(state: State<Arc<CombinedState>>) -> String {
    "Hello world! ".to_string() + &state.a.data
}

async fn get_handler_wrapper(
    state: State<Arc<CombinedState>>,
    header_map: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, io::Error>>>, Response> {
    tracing::info!("get_handler_wrapper");
    // instead of clone impl FromRef
    get_handler(state.app_state.clone(), header_map).await
}

async fn post_handler_wrapper(
    state: State<Arc<CombinedState>>,
    header_map: HeaderMap,
    message: Json<ClientJsonRpcMessage>,
) -> Result<Response, Response> {
    tracing::info!("post_handler_wrapper");
    // instead of clone impl FromRef
    post_handler(state.app_state.clone(), header_map, message).await
}

async fn delete_handler_wrapper(
    state: State<Arc<CombinedState>>,
    header_map: HeaderMap,
) -> Result<StatusCode, Response> {
    tracing::info!("delete_handler_wrapper");
    // instead of clone impl FromRef
    delete_handler(state.app_state.clone(), header_map).await
}

trait ExternalState {
    fn get_value() -> String;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ExternalA {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ExternalB {}

impl ExternalState for ExternalA {
    fn get_value() -> String {
        "A".to_string()
    }
}

impl ExternalState for ExternalB {
    fn get_value() -> String {
        "B".to_string()
    }
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

    let app_state = AppState {
        data: "state".to_string(),
    };

    let (app, transport_rx) = App::new(config.sse_keep_alive.unwrap_or(DEFAULT_AUTO_PING_INTERVAL));
    let combined_state = CombinedState {
        a: State(app_state),
        app_state: State(app.clone()),
    };
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
    let counter = Arc::new(Mutex::new(0));
    let external = Arc::new(crate::counter::MockDataService {});

    let counter = Counter::new(counter, external);
    sse_server.with_service(move || counter.clone());
    let router = Router::new()
        .route("/hello", get(hello_world))
        .nest_service("/mcp", sse_router)
        .with_state(Arc::from(combined_state));

    let app = router.into_make_service();
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
