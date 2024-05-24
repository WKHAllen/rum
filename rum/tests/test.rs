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
    my_query: String,
    my_query_opt: Option<String>,
    my_header: String,
    my_header_opt: Option<String>,
}

#[middleware]
async fn json_middleware(req: Request, next: NextFn) -> Response {
    next(req)
        .await
        .header("Content-Type", "application/json")
        .body_or("{}")
}

#[middleware]
async fn set_cookie_middleware(
    req: Request,
    next: NextFn,
    counter_cookie: CookieOptional<"counter", usize>,
) -> Response {
    let res = next(req).await;

    match counter_cookie.into_inner() {
        Some(count) => res.cookie(SetCookie::new("counter", count + 1)),
        None => res.cookie(SetCookie::new("counter", 1)),
    }
}

#[handler]
async fn greet(
    greeting_request: Json<GreetingRequest>,
    state: State<Counter>,
    my_query: QueryParam<"my_query">,
    my_query_opt: QueryParamOptional<"my_query_opt">,
    my_header: Header<"my_header">,
    my_header_opt: HeaderOptional<"my_header_opt">,
) -> Json<GreetingResponse> {
    let count = {
        let mut counter = state.count.lock().await;
        *counter += 1;
        *counter
    };

    Json(GreetingResponse {
        message: format!("Hello, {}!", greeting_request.name),
        count,
        my_query: my_query.into_inner(),
        my_query_opt: my_query_opt.into_inner(),
        my_header: my_header.into_inner(),
        my_header_opt: my_header_opt.into_inner(),
    })
}

#[tokio::test]
async fn test() {
    let addr = "127.0.0.1:3000";
    let (shutdown_sender, shutdown_receiver) = shutdown_signal();
    let (error_sender, mut error_receiver) = error_report_stream();

    spawn(async move {
        ctrl_c().await.unwrap();
        shutdown_sender.shutdown().await;
    });

    spawn(async move {
        while let Some(err) = error_receiver.next().await {
            dbg!(err);
        }
    });

    Server::new()
        .route_group(
            RouteGroup::new("/api/v1")
                .get("/greet", greet)
                .with_middleware(json_middleware)
                .with_middleware(set_cookie_middleware),
        )
        .with_state(Counter::default())
        .with_graceful_shutdown(shutdown_receiver)
        .with_error_reporting(error_sender)
        .serve(addr)
        .await
        .unwrap();
}
