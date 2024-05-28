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
        let listener = TcpListener::bind("127.0.0.1:3000").await?;
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
        method: HttpMethod,
        path: &str,
        config_fn: F,
    ) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        let req = self.client.request(
            method.into(),
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
        method: HttpMethod,
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
        self.query(HttpMethod::Get, path, config_fn).await
    }

    pub async fn get_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(HttpMethod::Get, path, config_fn).await
    }

    pub async fn post<F>(&self, path: &str, config_fn: F) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(HttpMethod::Post, path, config_fn).await
    }

    pub async fn post_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(HttpMethod::Post, path, config_fn).await
    }

    pub async fn put<F>(&self, path: &str, config_fn: F) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(HttpMethod::Put, path, config_fn).await
    }

    pub async fn put_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(HttpMethod::Put, path, config_fn).await
    }

    pub async fn patch<F>(&self, path: &str, config_fn: F) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(HttpMethod::Patch, path, config_fn).await
    }

    pub async fn patch_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(HttpMethod::Patch, path, config_fn).await
    }

    pub async fn delete<F>(&self, path: &str, config_fn: F) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(HttpMethod::Delete, path, config_fn).await
    }

    pub async fn delete_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(HttpMethod::Delete, path, config_fn).await
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
    let server = TestServer::new()
        .config(|server| server.get("/test", |_| async move { Response::new() }))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}
