mod counter;

use axum::extract::State;
use axum::{Router, routing::get};
use tokio::net::TcpListener;

#[derive(Clone)]
struct AppState {
    data: String,
}

async fn hello_world(app_state: State<AppState>) -> String {
    "Hello world! ".to_string() + &app_state.data
}

#[tokio::main]
async fn main() {
    let mcp = counter::Counter::new();

    let router = Router::new()
        .route("/hello", get(hello_world))
        .with_state(AppState {
            data: "XXX".to_string(),
        });

    let app = router.into_make_service();
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
