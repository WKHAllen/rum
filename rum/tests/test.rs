use rum::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::signal::ctrl_c;
use tokio::spawn;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Default)]
struct Counter {
    count: Arc<Mutex<usize>>,
}

#[derive(Serialize, Deserialize)]
struct GreetingRequest {
    name: String,
}

#[derive(Serialize, Deserialize)]
struct GreetingResponse {
    message: String,
    count: usize,
}

#[handler]
async fn greet(
    greeting_request: Json<GreetingRequest>,
    state: State<Counter>,
) -> Json<GreetingResponse> {
    let count = {
        let mut counter = state.count.lock().await;
        *counter += 1;
        *counter
    };

    Json(GreetingResponse {
        message: format!("Hello, {}!", greeting_request.name),
        count,
    })
}

#[tokio::test]
async fn test() {
    let addr = "127.0.0.1:3000";
    let (shutdown_sender, shutdown_receiver) = shutdown_signal();

    spawn(async move {
        ctrl_c().await.unwrap();
        shutdown_sender.shutdown().await;
    });

    Server::new()
        .route_group("/api/v1", RouteGroup::new().get("/greet", greet))
        .with_state(Counter::default())
        .with_graceful_shutdown(shutdown_receiver)
        .serve(addr)
        .await
        .unwrap();
}
