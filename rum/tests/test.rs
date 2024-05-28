use rum::error::Error;
use rum::prelude::*;
use serde::de::DeserializeOwned;
use std::io;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::spawn;
use tokio::task::JoinHandle;

struct TestServer {
    server: Server,
    shutdown_sender: ShutdownSender,
    error_receiver: ErrorReceiver,
}

impl TestServer {
    pub fn new() -> Self {
        let (shutdown_sender, shutdown_receiver) = shutdown_signal();
        let (error_sender, error_receiver) = error_report_stream();

        let server = Server::new()
            .with_graceful_shutdown(shutdown_receiver)
            .with_error_reporting(error_sender);

        Self {
            server,
            shutdown_sender,
            error_receiver,
        }
    }

    pub fn config<F>(mut self, config_fn: F) -> Self
    where
        F: FnOnce(Server) -> Server,
    {
        self.server = config_fn(self.server);
        self
    }

    pub async fn start(self) -> io::Result<TestServerHandle> {
        let server = self.server;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();

        let serve_task = spawn(async move { server.serve_with(listener).await });

        Ok(TestServerHandle {
            port,
            serve_task,
            shutdown_sender: self.shutdown_sender,
            error_receiver: self.error_receiver,
            client: reqwest::Client::new(),
        })
    }
}

struct TestServerHandle {
    port: u16,
    serve_task: JoinHandle<()>,
    shutdown_sender: ShutdownSender,
    error_receiver: ErrorReceiver,
    client: reqwest::Client,
}

#[allow(dead_code)]
impl TestServerHandle {
    pub async fn query<F>(
        &self,
        method: Method,
        path: &str,
        config_fn: F,
    ) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        let req = self.client.request(
            method,
            format!(
                "http://localhost:{}/{}",
                self.port,
                path.strip_prefix('/').unwrap_or_default()
            ),
        );

        let req = config_fn(req);

        req.send().await
    }

    pub async fn query_as<T, F>(
        &self,
        method: Method,
        path: &str,
        config_fn: F,
    ) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(method, path, config_fn).await?.json::<T>().await
    }

    pub async fn get<F>(&self, path: &str, config_fn: F) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(Method::GET, path, config_fn).await
    }

    pub async fn get_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(Method::GET, path, config_fn).await
    }

    pub async fn post<F>(&self, path: &str, config_fn: F) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(Method::POST, path, config_fn).await
    }

    pub async fn post_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(Method::POST, path, config_fn).await
    }

    pub async fn put<F>(&self, path: &str, config_fn: F) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(Method::PUT, path, config_fn).await
    }

    pub async fn put_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(Method::PUT, path, config_fn).await
    }

    pub async fn patch<F>(&self, path: &str, config_fn: F) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(Method::PATCH, path, config_fn).await
    }

    pub async fn patch_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(Method::PATCH, path, config_fn).await
    }

    pub async fn delete<F>(&self, path: &str, config_fn: F) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(Method::DELETE, path, config_fn).await
    }

    pub async fn delete_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(Method::DELETE, path, config_fn).await
    }

    pub async fn stop(mut self) -> Vec<Arc<Error>> {
        self.shutdown_sender.shutdown().await;
        self.serve_task.await.unwrap();

        let mut errors = Vec::new();

        while let Some(err) = self.error_receiver.next().await {
            errors.push(err);
        }

        errors
    }
}

macro_rules! assert_no_server_errors {
    ( $errors:expr ) => {
        if !$errors.is_empty() {
            panic!("test server produced errors:\n{:#?}", $errors);
        }
    };
}

#[tokio::test]
async fn test_automatic_200() {
    #[handler]
    async fn res_200() {}

    let server = TestServer::new()
        .config(|server| server.get("/test", res_200))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_automatic_400() {
    #[handler]
    async fn res_400(_: QueryParam<"missing">) {}

    let server = TestServer::new()
        .config(|server| server.get("/test", res_400))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_automatic_404() {
    let server = TestServer::new().start().await.unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_automatic_405() {
    #[handler]
    async fn res_405() {}

    let server = TestServer::new()
        .config(|server| server.get("/test", res_405))
        .start()
        .await
        .unwrap();

    let res = server.post("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_automatic_415() {
    #[handler]
    async fn res_415(_: Json<()>) {}

    let server = TestServer::new()
        .config(|server| server.get("/test", res_415))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| req.body("plain text body"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_automatic_500() {
    #[handler]
    async fn res_500(_: NextFn) {}

    let server = TestServer::new()
        .config(|server| server.get("/test", res_500))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let errors = server.stop().await;
    assert_eq!(errors.len(), 1);
    assert!(matches!(*errors[0], Error::NoNextFunction));
}
