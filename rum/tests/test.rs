use rum::error::{Error, Result};
use rum::prelude::*;
use rum::routing::{RoutePathMatchedSegment, RoutePathSegment};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::io;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
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

    pub async fn head<F>(&self, path: &str, config_fn: F) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(Method::HEAD, path, config_fn).await
    }

    pub async fn head_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(Method::HEAD, path, config_fn).await
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

    pub async fn connect<F>(&self, path: &str, config_fn: F) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(Method::CONNECT, path, config_fn).await
    }

    pub async fn connect_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(Method::CONNECT, path, config_fn).await
    }

    pub async fn options<F>(&self, path: &str, config_fn: F) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(Method::OPTIONS, path, config_fn).await
    }

    pub async fn options_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(Method::OPTIONS, path, config_fn).await
    }

    pub async fn trace<F>(&self, path: &str, config_fn: F) -> reqwest::Result<reqwest::Response>
    where
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query(Method::TRACE, path, config_fn).await
    }

    pub async fn trace_as<T, F>(&self, path: &str, config_fn: F) -> reqwest::Result<T>
    where
        T: DeserializeOwned,
        F: FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder,
    {
        self.query_as(Method::TRACE, path, config_fn).await
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

macro_rules! map {
    ( $( $k:expr => $v:expr ),* $(,)? ) => {{
        #[allow(unused_mut)]
        let mut tmp = ::std::collections::HashMap::new();
        $(
            tmp.insert($k, $v);
        )*
        tmp
    }};
}

macro_rules! set {
    ( $( $x:expr ),* $(,)? ) => {{
        #[allow(unused_mut)]
        let mut tmp = ::std::collections::HashSet::new();
        $(
            tmp.insert($x);
        )*
        tmp
    }};
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
        .config(|server| {
            server
                .get("/test", res_405)
                .head("/test", res_405)
                .delete("/test", res_405)
        })
        .start()
        .await
        .unwrap();

    let res = server.post("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::METHOD_NOT_ALLOWED);
    let allow_header = res
        .headers()
        .get(http::header::ALLOW)
        .unwrap()
        .to_str()
        .unwrap()
        .split(", ")
        .collect::<HashSet<_>>();
    let expected_allow_header = set!["GET", "HEAD", "DELETE"];
    assert_eq!(allow_header, expected_allow_header);

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

#[tokio::test]
async fn test_string_error() {
    async fn string_error(req: Request) -> Response {
        let maybe_string = BodyString::from_request(&req);
        assert!(matches!(maybe_string, Err(Error::StringError(_))));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", string_error))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| {
            req.header(http::header::CONTENT_TYPE, "text/plain")
                .body(vec![0, 159, 146, 150])
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_json_error() {
    async fn string_error(req: Request) -> Response {
        #[derive(Debug, Deserialize)]
        struct TestJson {
            _missing_field: i32,
        }

        let maybe_json = Json::<TestJson>::from_request(&req);
        assert!(matches!(maybe_json, Err(Error::JsonError(_))));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", string_error))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| {
            req.header(http::header::CONTENT_TYPE, "application/json")
                .body("{}")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_missing_path_param_error() {
    async fn missing_path_param(req: Request) -> Response {
        let path_params = PathParamMap::from_request(&req).unwrap();
        let maybe_path_param = path_params.get("missing");
        assert!(matches!(
            maybe_path_param,
            Err(Error::MissingPathParameterError(name)) if name.as_str() == "missing"
        ));

        let maybe_path_param = PathParam::<"missing">::from_request(&req);
        assert!(matches!(
            maybe_path_param,
            Err(Error::MissingPathParameterError(name)) if name.as_str() == "missing"
        ));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", missing_path_param))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_missing_query_param_error() {
    async fn missing_query_param(req: Request) -> Response {
        let query_params = QueryParamMap::from_request(&req).unwrap();
        let maybe_query_param = query_params.get("missing");
        assert!(matches!(
            maybe_query_param,
            Err(Error::MissingQueryParameterError(name)) if name.as_str() == "missing"
        ));

        let maybe_query_param = QueryParam::<"missing">::from_request(&req);
        assert!(matches!(
            maybe_query_param,
            Err(Error::MissingQueryParameterError(name)) if name.as_str() == "missing"
        ));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", missing_query_param))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_missing_header_error() {
    async fn missing_header(req: Request) -> Response {
        let headers = HeaderMap::from_request(&req).unwrap();
        let maybe_header = headers.get("missing");
        assert!(matches!(
            maybe_header,
            Err(Error::MissingHeaderError(name)) if name.as_str() == "missing"
        ));

        let maybe_header = Header::<"missing">::from_request(&req);
        assert!(matches!(
            maybe_header,
            Err(Error::MissingHeaderError(name)) if name.as_str() == "missing"
        ));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", missing_header))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_missing_cookie_error() {
    async fn missing_cookie(req: Request) -> Response {
        let cookies = CookieMap::from_request(&req).unwrap();
        let maybe_cookie = cookies.get("missing");
        assert!(matches!(
            maybe_cookie,
            Err(Error::MissingCookieError(name)) if name.as_str() == "missing"
        ));

        let maybe_cookie = Cookie::<"missing">::from_request(&req);
        assert!(matches!(
            maybe_cookie,
            Err(Error::MissingCookieError(name)) if name.as_str() == "missing"
        ));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", missing_cookie))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_path_param_parse_error() {
    async fn path_param_parse_error(req: Request) -> Response {
        let path_params = PathParamMap::from_request(&req).unwrap();
        let maybe_path_param = path_params.get_as::<i32>("parse_error");
        assert!(matches!(
            maybe_path_param,
            Err(Error::PathParameterParseError(name, _)) if name.as_str() == "parse_error"
        ));

        let maybe_path_param = PathParam::<"parse_error", i32>::from_request(&req);
        assert!(matches!(
            maybe_path_param,
            Err(Error::PathParameterParseError(name, _)) if name.as_str() == "parse_error"
        ));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test/{parse_error}", path_param_parse_error))
        .start()
        .await
        .unwrap();

    let res = server.get("/test/foo", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_query_param_parse_error() {
    async fn query_param_parse_error(req: Request) -> Response {
        let query_params = QueryParamMap::from_request(&req).unwrap();
        let maybe_query_param = query_params.get_as::<i32>("parse_error");
        assert!(matches!(
            maybe_query_param,
            Err(Error::QueryParameterParseError(name, _)) if name.as_str() == "parse_error"
        ));

        let maybe_query_param = QueryParam::<"parse_error", i32>::from_request(&req);
        assert!(matches!(
            maybe_query_param,
            Err(Error::QueryParameterParseError(name, _)) if name.as_str() == "parse_error"
        ));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", query_param_parse_error))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test?parse_error=foo", |req| req)
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_header_parse_error() {
    async fn header_parse_error(req: Request) -> Response {
        let headers = HeaderMap::from_request(&req).unwrap();
        let maybe_header = headers.get_as::<i32>("parse_error");
        assert!(matches!(
            maybe_header,
            Err(Error::HeaderParseError(name, _)) if name.as_str() == "parse_error"
        ));

        let maybe_header = Header::<"parse_error", i32>::from_request(&req);
        assert!(matches!(
            maybe_header,
            Err(Error::HeaderParseError(name, _)) if name.as_str() == "parse_error"
        ));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", header_parse_error))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| req.header("parse_error", "foo"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_cookie_parse_error() {
    async fn cookie_parse_error(req: Request) -> Response {
        let cookies = CookieMap::from_request(&req).unwrap();
        let maybe_cookie = cookies.get_as::<i32>("parse_error");
        assert!(matches!(
            maybe_cookie,
            Err(Error::CookieParseError(name, _)) if name.as_str() == "parse_error"
        ));

        let maybe_cookie = Cookie::<"parse_error", i32>::from_request(&req);
        assert!(matches!(
            maybe_cookie,
            Err(Error::CookieParseError(name, _)) if name.as_str() == "parse_error"
        ));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", cookie_parse_error))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| {
            req.header(http::header::COOKIE, "parse_error=foo")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_unknown_state_type_error() {
    async fn unknown_state_type_error(req: Request) -> Response {
        let maybe_state = State::<i32>::from_request(&req);
        assert!(matches!(
            maybe_state,
            Err(Error::UnknownStateTypeError("i32"))
        ));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", unknown_state_type_error))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_no_next_function() {
    async fn no_next_function(req: Request) -> Response {
        let maybe_next_function = NextFn::from_request(&req);
        assert!(matches!(maybe_next_function, Err(Error::NoNextFunction)));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", no_next_function))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_unsupported_media_type() {
    async fn unsupported_media_type_text(req: Request) -> Response {
        let maybe_text = BodyString::from_request(&req);
        assert!(matches!(maybe_text, Err(Error::UnsupportedMediaType)));

        Response::new()
    }

    async fn unsupported_media_type_json(req: Request) -> Response {
        #[derive(Deserialize)]
        struct TestJson;

        let maybe_json = Json::<TestJson>::from_request(&req);
        assert!(matches!(maybe_json, Err(Error::UnsupportedMediaType)));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| {
            server
                .get("/test/text", unsupported_media_type_text)
                .get("/test/json", unsupported_media_type_json)
        })
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test/text", |req| {
            req.header(http::header::CONTENT_TYPE, "application/json")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = server
        .get("/test/json", |req| {
            req.header(http::header::CONTENT_TYPE, "text/plain")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_server_json_error() {
    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct TestNum {
        num: usize,
    }

    #[handler]
    async fn server_json_error() -> Json<HashMap<TestNum, &str>> {
        Json(map! {
            TestNum { num: 1 } => "first",
            TestNum { num: 2 } => "second",
            TestNum { num: 3 } => "third",
        })
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", server_json_error))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let errors = server.stop().await;
    assert_eq!(errors.len(), 1);
    assert!(matches!(*errors[0], Error::ServerJsonError(_)));
}

#[tokio::test]
async fn test_request_methods() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    struct TestJson {
        num: i32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TestState {
        num: i32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TestLocalState {
        num: i32,
    }

    #[middleware]
    async fn request_methods_middleware(
        req: Request,
        local_state: LocalState,
        next: NextFn,
    ) -> Response {
        local_state
            .with(|state| state.insert(TestLocalState { num: 789 }))
            .await;
        next.call(req).await
    }

    async fn request_methods(req: Request) -> Response {
        let body_str = std::str::from_utf8(req.body()).unwrap();
        assert_eq!(body_str, "{\"num\":123}");

        let body_json = req.body_json::<TestJson>().unwrap();
        assert_eq!(body_json, TestJson { num: 123 });

        let method = req.method();
        assert_eq!(*method, Method::GET);

        let path = &*req.path();
        assert_eq!(
            path,
            &[
                RoutePathSegment::Static("test".to_owned()),
                RoutePathSegment::Static("234".to_owned())
            ]
        );

        let matched_path = &*req.matched_path();
        assert_eq!(
            matched_path,
            &[
                RoutePathMatchedSegment::Static("test".to_owned()),
                RoutePathMatchedSegment::Wildcard("num".to_owned(), "234".to_owned())
            ]
        );

        let path_param = req.path_param("num").unwrap();
        assert_eq!(path_param, "234");

        let invalid_path_param = req.path_param("invalid");
        assert!(invalid_path_param.is_err());

        let path_param_as = req.path_param_as::<i32>("num").unwrap();
        assert_eq!(path_param_as, 234);

        let query_param = req.query_param("num").unwrap();
        assert_eq!(query_param, "345");

        let invalid_query_param = req.query_param("invalid");
        assert!(invalid_query_param.is_err());

        let query_param_as = req.query_param_as::<i32>("num").unwrap();
        assert_eq!(query_param_as, 345);

        let query_param_optional = req.query_param_optional("num").unwrap();
        assert_eq!(query_param_optional, "345");

        let invalid_query_param_optional = req.query_param_optional("invalid");
        assert!(invalid_query_param_optional.is_none());

        let query_param_optional_as = req.query_param_optional_as::<i32>("num").unwrap().unwrap();
        assert_eq!(query_param_optional_as, 345);

        let header = req.header("num").unwrap();
        assert_eq!(header, &["456"]);

        let invalid_header = req.header("invalid");
        assert!(invalid_header.is_err());

        let header_as = req.header_as::<i32>("num").unwrap();
        assert_eq!(header_as, &[456]);

        let header_optional = req.header_optional("num").unwrap();
        assert_eq!(header_optional, &["456"]);

        let invalid_header_optional = req.header_optional("invalid");
        assert!(invalid_header_optional.is_none());

        let header_optional_as = req.header_optional_as::<i32>("num").unwrap().unwrap();
        assert_eq!(header_optional_as, &[456]);

        let cookie = req.cookie("num").unwrap();
        assert_eq!(cookie, "567");

        let invalid_cookie = req.cookie("invalid");
        assert!(invalid_cookie.is_err());

        let cookie_as = req.cookie_as::<i32>("num").unwrap();
        assert_eq!(cookie_as, 567);

        let cookie_optional = req.cookie_optional("num").unwrap();
        assert_eq!(cookie_optional, "567");

        let invalid_cookie_optional = req.cookie_optional("invalid");
        assert!(invalid_cookie_optional.is_none());

        let cookie_optional_as = req.cookie_optional_as::<i32>("num").unwrap().unwrap();
        assert_eq!(cookie_optional_as, 567);

        let state_value = req.state_value::<TestState>().unwrap();
        assert_eq!(state_value, TestState { num: 678 });

        let local_state = req.local_state();
        let local_state_value = local_state
            .with(|state| state.get_copied::<TestLocalState>())
            .await
            .unwrap();
        assert_eq!(local_state_value, TestLocalState { num: 789 });

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| {
            server
                .with_middleware(request_methods_middleware)
                .get("/test/{num}", request_methods)
                .with_state(TestState { num: 678 })
        })
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test/234?num=345", |req| {
            req.body("{\"num\":123}")
                .header("num", "456")
                .header(http::header::COOKIE, "num=567")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_request() {
    async fn extract_request(req: Request) -> Response {
        let req = Request::from_request(&req).unwrap();
        let body_str = req.body_str().unwrap();
        assert_eq!(body_str, "Hello, request!");

        let maybe_next_fn = req.next_fn();
        assert!(maybe_next_fn.is_none());

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_request))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| req.body("Hello, request!"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_body_raw() {
    async fn extract_body_raw(req: Request) -> Response {
        let body_raw = BodyRaw::from_request(&req).unwrap();
        assert_eq!(&*body_raw, &[0, 159, 146, 150]);

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_body_raw))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| req.body(vec![0, 159, 146, 150]))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_body_string() {
    async fn extract_body_string(req: Request) -> Response {
        let body_str = BodyString::from_request(&req).unwrap();
        assert_eq!(*body_str, "Hello, body string!");
        assert_eq!(body_str.into_inner(), "Hello, body string!");

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_body_string))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| {
            req.body("Hello, body string!")
                .header(http::header::CONTENT_TYPE, "text/plain")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_json() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    struct TestJson {
        num: i32,
    }

    async fn extract_json(req: Request) -> Response {
        let body_json = Json::<TestJson>::from_request(&req).unwrap();
        assert_eq!(*body_json, TestJson { num: 123 });
        assert_eq!(body_json.into_inner(), TestJson { num: 123 });

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_json))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| {
            req.body("{\"num\":123}")
                .header(http::header::CONTENT_TYPE, "application/json")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_method() {
    async fn extract_method(req: Request) -> Response {
        let method = Method::from_request(&req).unwrap();
        assert_eq!(method, Method::GET);

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_method))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_route_path_string() {
    async fn extract_route_path(req: Request) -> Response {
        let route_path_str = RoutePathString::from_request(&req).unwrap();
        assert_eq!(*route_path_str, "/test/123");
        assert_eq!(route_path_str.into_inner(), "/test/123");

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test/{num}", extract_route_path))
        .start()
        .await
        .unwrap();

    let res = server.get("/test/123", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_route_path() {
    async fn extract_route_path(req: Request) -> Response {
        let route_path = RoutePath::from_request(&req).unwrap();
        assert_eq!(
            &*route_path,
            &[
                RoutePathSegment::Static("test".to_owned()),
                RoutePathSegment::Static("123".to_owned())
            ]
        );

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test/{num}", extract_route_path))
        .start()
        .await
        .unwrap();

    let res = server.get("/test/123", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_route_path_matched() {
    async fn extract_route_path_matched(req: Request) -> Response {
        let route_path_matched = RoutePathMatched::from_request(&req).unwrap();
        assert_eq!(
            &*route_path_matched,
            &[
                RoutePathMatchedSegment::Static("test".to_owned()),
                RoutePathMatchedSegment::Wildcard("num".to_owned(), "123".to_owned())
            ]
        );

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test/{num}", extract_route_path_matched))
        .start()
        .await
        .unwrap();

    let res = server.get("/test/123", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_path_param_map() {
    async fn extract_path_param_map(req: Request) -> Response {
        let path_param_map = PathParamMap::from_request(&req).unwrap();
        assert_eq!(path_param_map.get("num").unwrap(), "123");
        assert_eq!(path_param_map.get_as::<i32>("num").unwrap(), 123);
        assert!(path_param_map.get("invalid").is_err());
        assert!(path_param_map.get_as::<i32>("invalid").is_err());

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test/{num}", extract_path_param_map))
        .start()
        .await
        .unwrap();

    let res = server.get("/test/123", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_path_params() {
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    struct TestPathParams {
        message: String,
    }

    async fn extract_path_params(req: Request) -> Response {
        let path_params = PathParams::<TestPathParams>::from_request(&req).unwrap();
        assert_eq!(
            *path_params,
            TestPathParams {
                message: "hello_path_params".to_owned()
            }
        );
        assert_eq!(
            path_params.into_inner(),
            TestPathParams {
                message: "hello_path_params".to_owned()
            }
        );

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test/{message}", extract_path_params))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test/hello_path_params", |req| req)
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_path_param() {
    async fn extract_path_param(req: Request) -> Response {
        let path_param = PathParam::<"num", i32>::from_request(&req).unwrap();
        assert_eq!(*path_param, 123);
        assert_eq!(path_param.into_inner(), 123);

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test/{num}", extract_path_param))
        .start()
        .await
        .unwrap();

    let res = server.get("/test/123", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_query_param_map() {
    async fn extract_query_param_map(req: Request) -> Response {
        let query_param_map = QueryParamMap::from_request(&req).unwrap();
        assert_eq!(query_param_map.get("num").unwrap(), "123");
        assert_eq!(query_param_map.get_as::<i32>("num").unwrap(), 123);
        assert_eq!(query_param_map.get_optional("num"), Some("123"));
        assert_eq!(
            query_param_map.get_optional_as::<i32>("num").unwrap(),
            Some(123)
        );
        assert!(query_param_map.get_optional_as::<bool>("num").is_err());
        assert_eq!(query_param_map.get_optional("invalid"), None);
        assert_eq!(
            query_param_map.get_optional_as::<i32>("invalid").unwrap(),
            None
        );

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_query_param_map))
        .start()
        .await
        .unwrap();

    let res = server.get("/test?num=123", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_query_params() {
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    struct TestQueryParams {
        message: String,
    }

    async fn extract_query_params(req: Request) -> Response {
        let query_params = QueryParams::<TestQueryParams>::from_request(&req).unwrap();
        assert_eq!(
            *query_params,
            TestQueryParams {
                message: "hello_query_params".to_owned()
            }
        );
        assert_eq!(
            query_params.into_inner(),
            TestQueryParams {
                message: "hello_query_params".to_owned()
            }
        );

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_query_params))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test?message=hello_query_params", |req| req)
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_query_param() {
    async fn extract_query_param(req: Request) -> Response {
        let query_param = QueryParam::<"num", i32>::from_request(&req).unwrap();
        assert_eq!(*query_param, 123);
        assert_eq!(query_param.into_inner(), 123);

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_query_param))
        .start()
        .await
        .unwrap();

    let res = server.get("/test?num=123", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_query_param_optional() {
    async fn extract_query_param(req: Request) -> Response {
        let query_param = QueryParamOptional::<"num", i32>::from_request(&req).unwrap();
        assert_eq!(*query_param, Some(123));
        assert_eq!(query_param.into_inner(), Some(123));

        let query_param_invalid = QueryParamOptional::<"invalid", i32>::from_request(&req).unwrap();
        assert_eq!(*query_param_invalid, None);

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_query_param))
        .start()
        .await
        .unwrap();

    let res = server.get("/test?num=123", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_header_map() {
    async fn extract_header_map(req: Request) -> Response {
        let header_map = HeaderMap::from_request(&req).unwrap();
        assert_eq!(header_map.get("num").unwrap(), &["123"]);
        assert_eq!(header_map.get_as::<i32>("num").unwrap(), vec![123]);
        assert_eq!(header_map.get_optional("num").unwrap(), &["123".to_owned()]);
        assert_eq!(
            header_map.get_optional_as::<i32>("num").unwrap(),
            Some(vec![123])
        );
        assert!(header_map.get_optional_as::<bool>("num").is_err());
        assert_eq!(header_map.get_optional("invalid"), None);
        assert_eq!(header_map.get_optional_as::<i32>("invalid").unwrap(), None);

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_header_map))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| req.header("num", "123"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_headers() {
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    struct TestHeaders {
        message: Vec<String>,
    }

    async fn extract_headers(req: Request) -> Response {
        let headers = Headers::<TestHeaders>::from_request(&req).unwrap();
        assert_eq!(
            *headers,
            TestHeaders {
                message: vec!["hello_headers".to_owned()]
            }
        );
        assert_eq!(
            headers.into_inner(),
            TestHeaders {
                message: vec!["hello_headers".to_owned()]
            }
        );

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_headers))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| req.header("message", "hello_headers"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_header() {
    async fn extract_header(req: Request) -> Response {
        let header = Header::<"num", i32>::from_request(&req).unwrap();
        assert_eq!(*header, &[123]);
        assert_eq!(header.into_inner(), &[123]);

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_header))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| req.header("num", "123"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_header_optional() {
    async fn extract_header(req: Request) -> Response {
        let header = HeaderOptional::<"num", i32>::from_request(&req).unwrap();
        assert_eq!(*header, Some(vec![123]));
        assert_eq!(header.into_inner(), Some(vec![123]));

        let header_invalid = HeaderOptional::<"invalid", i32>::from_request(&req).unwrap();
        assert_eq!(*header_invalid, None);

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_header))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| req.header("num", "123"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_cookie_map() {
    async fn extract_cookie_map(req: Request) -> Response {
        let cookie_map = CookieMap::from_request(&req).unwrap();
        assert_eq!(cookie_map.get("num").unwrap(), "123");
        assert_eq!(cookie_map.get_as::<i32>("num").unwrap(), 123);
        assert_eq!(cookie_map.get_optional("num"), Some("123"));
        assert_eq!(cookie_map.get_optional_as::<i32>("num").unwrap(), Some(123));
        assert!(cookie_map.get_optional_as::<bool>("num").is_err());
        assert_eq!(cookie_map.get_optional("invalid"), None);
        assert_eq!(cookie_map.get_optional_as::<i32>("invalid").unwrap(), None);

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_cookie_map))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| req.header(http::header::COOKIE, "num=123"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_cookies() {
    #[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
    struct TestCookies {
        message: String,
    }

    async fn extract_cookies(req: Request) -> Response {
        let cookies = Cookies::<TestCookies>::from_request(&req).unwrap();
        assert_eq!(
            *cookies,
            TestCookies {
                message: "hello_cookies".to_owned()
            }
        );
        assert_eq!(
            cookies.into_inner(),
            TestCookies {
                message: "hello_cookies".to_owned()
            }
        );

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_cookies))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| {
            req.header(http::header::COOKIE, "message=hello_cookies")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_cookie() {
    async fn extract_cookie(req: Request) -> Response {
        let cookie = Cookie::<"num", i32>::from_request(&req).unwrap();
        assert_eq!(*cookie, 123);
        assert_eq!(cookie.into_inner(), 123);

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_cookie))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| req.header(http::header::COOKIE, "num=123"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_cookie_optional() {
    async fn extract_cookie(req: Request) -> Response {
        let cookie = CookieOptional::<"num", i32>::from_request(&req).unwrap();
        assert_eq!(*cookie, Some(123));
        assert_eq!(cookie.into_inner(), Some(123));

        let cookie_invalid = CookieOptional::<"invalid", i32>::from_request(&req).unwrap();
        assert_eq!(*cookie_invalid, None);

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", extract_cookie))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| req.header(http::header::COOKIE, "num=123"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_state() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TestState {
        num: i32,
    }

    async fn extract_state(req: Request) -> Response {
        let state = State::<TestState>::from_request(&req).unwrap();
        assert_eq!(*state, TestState { num: 123 });
        assert_eq!(state.into_inner(), TestState { num: 123 });

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| {
            server
                .get("/test", extract_state)
                .with_state(TestState { num: 123 })
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_local_state() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TestLocalState {
        num: i32,
    }

    #[middleware]
    async fn extract_local_state_middleware(
        req: Request,
        local_state: LocalState,
        next: NextFn,
    ) -> Response {
        local_state
            .with(|state| state.insert(TestLocalState { num: 123 }))
            .await;
        next.call(req).await
    }

    async fn extract_local_state(req: Request) -> Response {
        let local_state = LocalState::from_request(&req).unwrap();
        let local_state_value = local_state
            .with(|state| state.get_copied::<TestLocalState>())
            .await
            .unwrap();
        assert_eq!(local_state_value, TestLocalState { num: 123 });

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| {
            server
                .with_middleware(extract_local_state_middleware)
                .get("/test", extract_local_state)
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_extract_next_fn() {
    async fn extract_next_fn_middleware(req: Request) -> Response {
        let next = NextFn::from_request(&req).unwrap();
        next.call(req).await
    }

    async fn extract_next_fn(req: Request) -> Response {
        let maybe_next = NextFn::from_request(&req);
        assert!(matches!(maybe_next, Err(Error::NoNextFunction)));

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| {
            server
                .with_middleware(extract_next_fn_middleware)
                .get("/test", extract_next_fn)
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_route_path() {
    assert_eq!(RoutePath::new().to_string(), "/");
    assert_eq!(RoutePath::from("").to_string(), "/");
    assert_eq!(RoutePath::from("/").to_string(), "/");
    assert_eq!(RoutePath::from("test").to_string(), "/test");
    assert_eq!(RoutePath::from("/test").to_string(), "/test");
    assert_eq!(RoutePath::from("/test/").to_string(), "/test");
    assert_eq!(RoutePath::from("test/123").to_string(), "/test/123");
    assert_eq!(RoutePath::from("/test/123").to_string(), "/test/123");
    assert_eq!(RoutePath::from("/test/123/").to_string(), "/test/123");

    assert_eq!(
        RoutePath::from("/test").join("/123").to_string(),
        "/test/123"
    );
    assert_eq!(
        RoutePath::from("foo").join("bar").join("baz").to_string(),
        "/foo/bar/baz"
    );

    assert_eq!(
        RoutePath::from_iter(RoutePath::from("/foo/bar/baz").iter()),
        RoutePath::from("/foo/bar/baz")
    );

    assert_eq!(RoutePath::new().num_segments(), 0);
    assert_eq!(RoutePath::from("").num_segments(), 0);
    assert_eq!(RoutePath::from("/").num_segments(), 0);
    assert_eq!(RoutePath::from("test").num_segments(), 1);
    assert_eq!(RoutePath::from("/test").num_segments(), 1);
    assert_eq!(RoutePath::from("/test/").num_segments(), 1);
    assert_eq!(RoutePath::from("test/123").num_segments(), 2);
    assert_eq!(RoutePath::from("/test/123").num_segments(), 2);
    assert_eq!(RoutePath::from("/test/123/").num_segments(), 2);

    assert_eq!(RoutePath::new().segments(), &[]);
    assert_eq!(RoutePath::from("").segments(), &[]);
    assert_eq!(RoutePath::from("/").segments(), &[]);
    assert_eq!(
        RoutePath::from("test").segments(),
        &[RoutePathSegment::Static("test".to_owned())]
    );
    assert_eq!(
        RoutePath::from("/test").segments(),
        &[RoutePathSegment::Static("test".to_owned())]
    );
    assert_eq!(
        RoutePath::from("/test/").segments(),
        &[RoutePathSegment::Static("test".to_owned())]
    );
    assert_eq!(
        RoutePath::from("test/123").segments(),
        &[
            RoutePathSegment::Static("test".to_owned()),
            RoutePathSegment::Static("123".to_owned())
        ]
    );
    assert_eq!(
        RoutePath::from("/test/123").segments(),
        &[
            RoutePathSegment::Static("test".to_owned()),
            RoutePathSegment::Static("123".to_owned())
        ]
    );
    assert_eq!(
        RoutePath::from("/test/123/").segments(),
        &[
            RoutePathSegment::Static("test".to_owned()),
            RoutePathSegment::Static("123".to_owned())
        ]
    );

    assert_eq!(
        RoutePath::from("/foo/bar/baz").with_segments(..),
        &[
            RoutePathSegment::Static("foo".to_owned()),
            RoutePathSegment::Static("bar".to_owned()),
            RoutePathSegment::Static("baz".to_owned())
        ]
    );
    assert_eq!(
        RoutePath::from("/foo/bar/baz").with_segments(1..),
        &[
            RoutePathSegment::Static("bar".to_owned()),
            RoutePathSegment::Static("baz".to_owned())
        ]
    );
    assert_eq!(
        RoutePath::from("/foo/bar/baz").with_segments(..2),
        &[
            RoutePathSegment::Static("foo".to_owned()),
            RoutePathSegment::Static("bar".to_owned())
        ]
    );
    assert_eq!(
        RoutePath::from("/foo/bar/baz").with_segments(..=2),
        &[
            RoutePathSegment::Static("foo".to_owned()),
            RoutePathSegment::Static("bar".to_owned()),
            RoutePathSegment::Static("baz".to_owned())
        ]
    );
    assert_eq!(
        RoutePath::from("/foo/bar/baz").with_segments(1..2),
        &[RoutePathSegment::Static("bar".to_owned())]
    );
    assert_eq!(
        RoutePath::from("/foo/bar/baz").with_segments(1..=2),
        &[
            RoutePathSegment::Static("bar".to_owned()),
            RoutePathSegment::Static("baz".to_owned())
        ]
    );

    assert_eq!(
        RoutePath::from("/foo/bar/baz").of_segments(..),
        RoutePath::from("/foo/bar/baz")
    );
    assert_eq!(
        RoutePath::from("/foo/bar/baz").of_segments(1..),
        RoutePath::from("/bar/baz")
    );
    assert_eq!(
        RoutePath::from("/foo/bar/baz").of_segments(..2),
        RoutePath::from("/foo/bar")
    );
    assert_eq!(
        RoutePath::from("/foo/bar/baz").of_segments(..=2),
        RoutePath::from("/foo/bar/baz")
    );
    assert_eq!(
        RoutePath::from("/foo/bar/baz").of_segments(1..2),
        RoutePath::from("/bar")
    );
    assert_eq!(
        RoutePath::from("/foo/bar/baz").of_segments(1..=2),
        RoutePath::from("/bar/baz")
    );

    assert_eq!(RoutePath::from("/").split_first(), None);
    assert_eq!(
        RoutePath::from("/foo").split_first(),
        Some((
            RoutePathSegment::Static("foo".to_owned()),
            RoutePath::from("/")
        ))
    );
    assert_eq!(
        RoutePath::from("/foo/bar").split_first(),
        Some((
            RoutePathSegment::Static("foo".to_owned()),
            RoutePath::from("/bar")
        ))
    );
    assert_eq!(
        RoutePath::from("/foo/bar/baz").split_first(),
        Some((
            RoutePathSegment::Static("foo".to_owned()),
            RoutePath::from("/bar/baz")
        ))
    );
}

#[tokio::test]
async fn test_route_path_matched() {
    assert_eq!(
        RoutePathMatched::from(RoutePathMatchedSegment::Static("test".to_owned())).join(
            RoutePathMatchedSegment::Wildcard("num".to_owned(), "/123".to_owned())
        ),
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("test".to_owned()),
            RoutePathMatchedSegment::Wildcard("num".to_owned(), "/123".to_owned())
        ])
    );
    assert_eq!(
        RoutePathMatched::from(RoutePathMatchedSegment::Static("foo".to_owned()))
            .join(RoutePathMatchedSegment::Static("bar".to_owned()))
            .join(RoutePathMatchedSegment::Static("baz".to_owned())),
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
    );

    assert_eq!(
        RoutePathMatched::from_iter(
            RoutePathMatched::from([
                RoutePathMatchedSegment::Static("foo".to_owned()),
                RoutePathMatchedSegment::Static("bar".to_owned()),
                RoutePathMatchedSegment::Static("baz".to_owned()),
            ])
            .iter()
        ),
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
    );

    assert_eq!(RoutePathMatched::new().num_segments(), 0);
    assert_eq!(RoutePathMatched::from([]).num_segments(), 0);
    assert_eq!(
        RoutePathMatched::from([RoutePathMatchedSegment::Static("test".to_owned())]).num_segments(),
        1
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("test".to_owned()),
            RoutePathMatchedSegment::Wildcard("num".to_owned(), "123".to_owned())
        ])
        .num_segments(),
        2
    );

    assert_eq!(RoutePathMatched::new().segments(), &[]);
    assert_eq!(RoutePathMatched::from([]).segments(), &[]);
    assert_eq!(
        RoutePathMatched::from([RoutePathMatchedSegment::Static("test".to_owned())]).segments(),
        &[RoutePathMatchedSegment::Static("test".to_owned())]
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("test".to_owned()),
            RoutePathMatchedSegment::Wildcard("num".to_owned(), "123".to_owned())
        ])
        .segments(),
        &[
            RoutePathMatchedSegment::Static("test".to_owned()),
            RoutePathMatchedSegment::Wildcard("num".to_owned(), "123".to_owned())
        ]
    );

    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
        .with_segments(..),
        &[
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned())
        ]
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
        .with_segments(1..),
        &[
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned())
        ]
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
        .with_segments(..2),
        &[
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned())
        ]
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
        .with_segments(..=2),
        &[
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned())
        ]
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
        .with_segments(1..2),
        &[RoutePathMatchedSegment::Static("bar".to_owned())]
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
        .with_segments(1..=2),
        &[
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned())
        ]
    );

    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
        .of_segments(..),
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
        .of_segments(1..),
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
        .of_segments(..2),
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
        ])
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
        .of_segments(..=2),
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
        .of_segments(1..2),
        RoutePathMatched::from([RoutePathMatchedSegment::Static("bar".to_owned()),])
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
        .of_segments(1..=2),
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
    );

    assert_eq!(RoutePathMatched::from([]).split_first(), None);
    assert_eq!(
        RoutePathMatched::from([RoutePathMatchedSegment::Static("foo".to_owned()),]).split_first(),
        Some((
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatched::from([])
        ))
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
        ])
        .split_first(),
        Some((
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatched::from([RoutePathMatchedSegment::Static("bar".to_owned()),])
        ))
    );
    assert_eq!(
        RoutePathMatched::from([
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatchedSegment::Static("bar".to_owned()),
            RoutePathMatchedSegment::Static("baz".to_owned()),
        ])
        .split_first(),
        Some((
            RoutePathMatchedSegment::Static("foo".to_owned()),
            RoutePathMatched::from([
                RoutePathMatchedSegment::Static("bar".to_owned()),
                RoutePathMatchedSegment::Static("baz".to_owned()),
            ])
        ))
    );
}

#[tokio::test]
async fn test_path_param_parsing() {
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestCustom(String);

    impl FromStr for TestCustom {
        type Err = Infallible;

        fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
            Ok(Self(s.to_uppercase()))
        }
    }

    async fn path_param_parse(req: Request) -> Response {
        assert!(matches!(
            PathParam::<"bool", bool>::from_request(&req).map(PathParam::into_inner),
            Ok(true)
        ));
        assert!(PathParam::<"bool", char>::from_request(&req).is_err());
        assert!(PathParam::<"bool", f32>::from_request(&req).is_err());
        assert!(PathParam::<"bool", f64>::from_request(&req).is_err());
        assert!(PathParam::<"bool", i8>::from_request(&req).is_err());
        assert!(PathParam::<"bool", i16>::from_request(&req).is_err());
        assert!(PathParam::<"bool", i32>::from_request(&req).is_err());
        assert!(PathParam::<"bool", i64>::from_request(&req).is_err());
        assert!(PathParam::<"bool", i128>::from_request(&req).is_err());
        assert!(PathParam::<"bool", isize>::from_request(&req).is_err());
        assert!(PathParam::<"bool", u8>::from_request(&req).is_err());
        assert!(PathParam::<"bool", u16>::from_request(&req).is_err());
        assert!(PathParam::<"bool", u32>::from_request(&req).is_err());
        assert!(PathParam::<"bool", u64>::from_request(&req).is_err());
        assert!(PathParam::<"bool", u128>::from_request(&req).is_err());
        assert!(PathParam::<"bool", usize>::from_request(&req).is_err());
        assert!(matches!(
            PathParam::<"bool", String>::from_request(&req).map(PathParam::into_inner),
            Ok(value) if value == "true"
        ));
        assert!(
            matches!(PathParam::<"bool", TestCustom>::from_request(&req).map(PathParam::into_inner), Ok(value) if value.0 == "TRUE")
        );

        assert!(PathParam::<"char", bool>::from_request(&req).is_err());
        assert!(matches!(
            PathParam::<"char", char>::from_request(&req).map(PathParam::into_inner),
            Ok('c')
        ));
        assert!(PathParam::<"char", f32>::from_request(&req).is_err());
        assert!(PathParam::<"char", f64>::from_request(&req).is_err());
        assert!(PathParam::<"char", i8>::from_request(&req).is_err());
        assert!(PathParam::<"char", i16>::from_request(&req).is_err());
        assert!(PathParam::<"char", i32>::from_request(&req).is_err());
        assert!(PathParam::<"char", i64>::from_request(&req).is_err());
        assert!(PathParam::<"char", i128>::from_request(&req).is_err());
        assert!(PathParam::<"char", isize>::from_request(&req).is_err());
        assert!(PathParam::<"char", u8>::from_request(&req).is_err());
        assert!(PathParam::<"char", u16>::from_request(&req).is_err());
        assert!(PathParam::<"char", u32>::from_request(&req).is_err());
        assert!(PathParam::<"char", u64>::from_request(&req).is_err());
        assert!(PathParam::<"char", u128>::from_request(&req).is_err());
        assert!(PathParam::<"char", usize>::from_request(&req).is_err());
        assert!(matches!(
            PathParam::<"char", String>::from_request(&req).map(PathParam::into_inner),
            Ok(value) if value == "c"
        ));
        assert!(
            matches!(PathParam::<"char", TestCustom>::from_request(&req).map(PathParam::into_inner), Ok(value) if value.0 == "C")
        );

        assert!(PathParam::<"int", bool>::from_request(&req).is_err());
        assert!(PathParam::<"int", char>::from_request(&req).is_err());
        assert!(matches!(
            PathParam::<"int", f32>::from_request(&req).map(PathParam::into_inner),
            Ok(1729.0)
        ));
        assert!(matches!(
            PathParam::<"int", f64>::from_request(&req).map(PathParam::into_inner),
            Ok(1729.0)
        ));
        assert!(PathParam::<"int", i8>::from_request(&req).is_err());
        assert!(matches!(
            PathParam::<"int", i16>::from_request(&req).map(PathParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            PathParam::<"int", i32>::from_request(&req).map(PathParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            PathParam::<"int", i64>::from_request(&req).map(PathParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            PathParam::<"int", i128>::from_request(&req).map(PathParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            PathParam::<"int", isize>::from_request(&req).map(PathParam::into_inner),
            Ok(1729)
        ));
        assert!(PathParam::<"int", u8>::from_request(&req).is_err());
        assert!(matches!(
            PathParam::<"int", u16>::from_request(&req).map(PathParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            PathParam::<"int", u32>::from_request(&req).map(PathParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            PathParam::<"int", u64>::from_request(&req).map(PathParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            PathParam::<"int", u128>::from_request(&req).map(PathParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            PathParam::<"int", usize>::from_request(&req).map(PathParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            PathParam::<"int", String>::from_request(&req).map(PathParam::into_inner),
            Ok(value) if value == "1729"
        ));
        assert!(
            matches!(PathParam::<"int", TestCustom>::from_request(&req).map(PathParam::into_inner), Ok(value) if value.0 == "1729")
        );

        assert!(PathParam::<"float", bool>::from_request(&req).is_err());
        assert!(PathParam::<"float", char>::from_request(&req).is_err());
        assert!(matches!(
            PathParam::<"float", f32>::from_request(&req).map(PathParam::into_inner),
            Ok(1.618)
        ));
        assert!(matches!(
            PathParam::<"float", f64>::from_request(&req).map(PathParam::into_inner),
            Ok(1.618)
        ));
        assert!(PathParam::<"float", i8>::from_request(&req).is_err());
        assert!(PathParam::<"float", i16>::from_request(&req).is_err());
        assert!(PathParam::<"float", i32>::from_request(&req).is_err());
        assert!(PathParam::<"float", i64>::from_request(&req).is_err());
        assert!(PathParam::<"float", i128>::from_request(&req).is_err());
        assert!(PathParam::<"float", isize>::from_request(&req).is_err());
        assert!(PathParam::<"float", u8>::from_request(&req).is_err());
        assert!(PathParam::<"float", u16>::from_request(&req).is_err());
        assert!(PathParam::<"float", u32>::from_request(&req).is_err());
        assert!(PathParam::<"float", u64>::from_request(&req).is_err());
        assert!(PathParam::<"float", u128>::from_request(&req).is_err());
        assert!(PathParam::<"float", usize>::from_request(&req).is_err());
        assert!(matches!(
            PathParam::<"float", String>::from_request(&req).map(PathParam::into_inner),
            Ok(value) if value == "1.618"
        ));
        assert!(
            matches!(PathParam::<"float", TestCustom>::from_request(&req).map(PathParam::into_inner), Ok(value) if value.0 == "1.618")
        );

        assert!(PathParam::<"string", bool>::from_request(&req).is_err());
        assert!(PathParam::<"string", char>::from_request(&req).is_err());
        assert!(PathParam::<"string", f32>::from_request(&req).is_err());
        assert!(PathParam::<"string", f64>::from_request(&req).is_err());
        assert!(PathParam::<"string", i8>::from_request(&req).is_err());
        assert!(PathParam::<"string", i16>::from_request(&req).is_err());
        assert!(PathParam::<"string", i32>::from_request(&req).is_err());
        assert!(PathParam::<"string", i64>::from_request(&req).is_err());
        assert!(PathParam::<"string", i128>::from_request(&req).is_err());
        assert!(PathParam::<"string", isize>::from_request(&req).is_err());
        assert!(PathParam::<"string", u8>::from_request(&req).is_err());
        assert!(PathParam::<"string", u16>::from_request(&req).is_err());
        assert!(PathParam::<"string", u32>::from_request(&req).is_err());
        assert!(PathParam::<"string", u64>::from_request(&req).is_err());
        assert!(PathParam::<"string", u128>::from_request(&req).is_err());
        assert!(PathParam::<"string", usize>::from_request(&req).is_err());
        assert!(matches!(
            PathParam::<"string", String>::from_request(&req).map(PathParam::into_inner),
            Ok(value) if value == "Rust"
        ));
        assert!(
            matches!(PathParam::<"string", TestCustom>::from_request(&req).map(PathParam::into_inner), Ok(value) if value.0 == "RUST")
        );

        assert!(PathParam::<"custom", bool>::from_request(&req).is_err());
        assert!(PathParam::<"custom", char>::from_request(&req).is_err());
        assert!(PathParam::<"custom", f32>::from_request(&req).is_err());
        assert!(PathParam::<"custom", f64>::from_request(&req).is_err());
        assert!(PathParam::<"custom", i8>::from_request(&req).is_err());
        assert!(PathParam::<"custom", i16>::from_request(&req).is_err());
        assert!(PathParam::<"custom", i32>::from_request(&req).is_err());
        assert!(PathParam::<"custom", i64>::from_request(&req).is_err());
        assert!(PathParam::<"custom", i128>::from_request(&req).is_err());
        assert!(PathParam::<"custom", isize>::from_request(&req).is_err());
        assert!(PathParam::<"custom", u8>::from_request(&req).is_err());
        assert!(PathParam::<"custom", u16>::from_request(&req).is_err());
        assert!(PathParam::<"custom", u32>::from_request(&req).is_err());
        assert!(PathParam::<"custom", u64>::from_request(&req).is_err());
        assert!(PathParam::<"custom", u128>::from_request(&req).is_err());
        assert!(PathParam::<"custom", usize>::from_request(&req).is_err());
        assert!(matches!(
            PathParam::<"custom", String>::from_request(&req).map(PathParam::into_inner),
            Ok(value) if value == "hello"
        ));
        assert!(
            matches!(PathParam::<"custom", TestCustom>::from_request(&req).map(PathParam::into_inner), Ok(value) if value.0 == "HELLO")
        );

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| {
            server.get(
                "/test/{bool}/{char}/{int}/{float}/{string}/{custom}",
                path_param_parse,
            )
        })
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test/true/c/1729/1.618/Rust/hello", |req| req)
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_query_param_parsing() {
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestCustom(String);

    impl FromStr for TestCustom {
        type Err = Infallible;

        fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
            Ok(Self(s.to_uppercase()))
        }
    }

    async fn query_param_parse(req: Request) -> Response {
        assert!(matches!(
            QueryParam::<"bool", bool>::from_request(&req).map(QueryParam::into_inner),
            Ok(true)
        ));
        assert!(QueryParam::<"bool", char>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", f32>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", f64>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", i8>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", i16>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", i32>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", i64>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", i128>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", isize>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", u8>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", u16>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", u32>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", u64>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", u128>::from_request(&req).is_err());
        assert!(QueryParam::<"bool", usize>::from_request(&req).is_err());
        assert!(matches!(
            QueryParam::<"bool", String>::from_request(&req).map(QueryParam::into_inner),
            Ok(value) if value == "true"
        ));
        assert!(
            matches!(QueryParam::<"bool", TestCustom>::from_request(&req).map(QueryParam::into_inner), Ok(value) if value.0 == "TRUE")
        );

        assert!(QueryParam::<"char", bool>::from_request(&req).is_err());
        assert!(matches!(
            QueryParam::<"char", char>::from_request(&req).map(QueryParam::into_inner),
            Ok('c')
        ));
        assert!(QueryParam::<"char", f32>::from_request(&req).is_err());
        assert!(QueryParam::<"char", f64>::from_request(&req).is_err());
        assert!(QueryParam::<"char", i8>::from_request(&req).is_err());
        assert!(QueryParam::<"char", i16>::from_request(&req).is_err());
        assert!(QueryParam::<"char", i32>::from_request(&req).is_err());
        assert!(QueryParam::<"char", i64>::from_request(&req).is_err());
        assert!(QueryParam::<"char", i128>::from_request(&req).is_err());
        assert!(QueryParam::<"char", isize>::from_request(&req).is_err());
        assert!(QueryParam::<"char", u8>::from_request(&req).is_err());
        assert!(QueryParam::<"char", u16>::from_request(&req).is_err());
        assert!(QueryParam::<"char", u32>::from_request(&req).is_err());
        assert!(QueryParam::<"char", u64>::from_request(&req).is_err());
        assert!(QueryParam::<"char", u128>::from_request(&req).is_err());
        assert!(QueryParam::<"char", usize>::from_request(&req).is_err());
        assert!(matches!(
            QueryParam::<"char", String>::from_request(&req).map(QueryParam::into_inner),
            Ok(value) if value == "c"
        ));
        assert!(
            matches!(QueryParam::<"char", TestCustom>::from_request(&req).map(QueryParam::into_inner), Ok(value) if value.0 == "C")
        );

        assert!(QueryParam::<"int", bool>::from_request(&req).is_err());
        assert!(QueryParam::<"int", char>::from_request(&req).is_err());
        assert!(matches!(
            QueryParam::<"int", f32>::from_request(&req).map(QueryParam::into_inner),
            Ok(1729.0)
        ));
        assert!(matches!(
            QueryParam::<"int", f64>::from_request(&req).map(QueryParam::into_inner),
            Ok(1729.0)
        ));
        assert!(QueryParam::<"int", i8>::from_request(&req).is_err());
        assert!(matches!(
            QueryParam::<"int", i16>::from_request(&req).map(QueryParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            QueryParam::<"int", i32>::from_request(&req).map(QueryParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            QueryParam::<"int", i64>::from_request(&req).map(QueryParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            QueryParam::<"int", i128>::from_request(&req).map(QueryParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            QueryParam::<"int", isize>::from_request(&req).map(QueryParam::into_inner),
            Ok(1729)
        ));
        assert!(QueryParam::<"int", u8>::from_request(&req).is_err());
        assert!(matches!(
            QueryParam::<"int", u16>::from_request(&req).map(QueryParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            QueryParam::<"int", u32>::from_request(&req).map(QueryParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            QueryParam::<"int", u64>::from_request(&req).map(QueryParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            QueryParam::<"int", u128>::from_request(&req).map(QueryParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            QueryParam::<"int", usize>::from_request(&req).map(QueryParam::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            QueryParam::<"int", String>::from_request(&req).map(QueryParam::into_inner),
            Ok(value) if value == "1729"
        ));
        assert!(
            matches!(QueryParam::<"int", TestCustom>::from_request(&req).map(QueryParam::into_inner), Ok(value) if value.0 == "1729")
        );

        assert!(QueryParam::<"float", bool>::from_request(&req).is_err());
        assert!(QueryParam::<"float", char>::from_request(&req).is_err());
        assert!(matches!(
            QueryParam::<"float", f32>::from_request(&req).map(QueryParam::into_inner),
            Ok(1.618)
        ));
        assert!(matches!(
            QueryParam::<"float", f64>::from_request(&req).map(QueryParam::into_inner),
            Ok(1.618)
        ));
        assert!(QueryParam::<"float", i8>::from_request(&req).is_err());
        assert!(QueryParam::<"float", i16>::from_request(&req).is_err());
        assert!(QueryParam::<"float", i32>::from_request(&req).is_err());
        assert!(QueryParam::<"float", i64>::from_request(&req).is_err());
        assert!(QueryParam::<"float", i128>::from_request(&req).is_err());
        assert!(QueryParam::<"float", isize>::from_request(&req).is_err());
        assert!(QueryParam::<"float", u8>::from_request(&req).is_err());
        assert!(QueryParam::<"float", u16>::from_request(&req).is_err());
        assert!(QueryParam::<"float", u32>::from_request(&req).is_err());
        assert!(QueryParam::<"float", u64>::from_request(&req).is_err());
        assert!(QueryParam::<"float", u128>::from_request(&req).is_err());
        assert!(QueryParam::<"float", usize>::from_request(&req).is_err());
        assert!(matches!(
            QueryParam::<"float", String>::from_request(&req).map(QueryParam::into_inner),
            Ok(value) if value == "1.618"
        ));
        assert!(
            matches!(QueryParam::<"float", TestCustom>::from_request(&req).map(QueryParam::into_inner), Ok(value) if value.0 == "1.618")
        );

        assert!(QueryParam::<"string", bool>::from_request(&req).is_err());
        assert!(QueryParam::<"string", char>::from_request(&req).is_err());
        assert!(QueryParam::<"string", f32>::from_request(&req).is_err());
        assert!(QueryParam::<"string", f64>::from_request(&req).is_err());
        assert!(QueryParam::<"string", i8>::from_request(&req).is_err());
        assert!(QueryParam::<"string", i16>::from_request(&req).is_err());
        assert!(QueryParam::<"string", i32>::from_request(&req).is_err());
        assert!(QueryParam::<"string", i64>::from_request(&req).is_err());
        assert!(QueryParam::<"string", i128>::from_request(&req).is_err());
        assert!(QueryParam::<"string", isize>::from_request(&req).is_err());
        assert!(QueryParam::<"string", u8>::from_request(&req).is_err());
        assert!(QueryParam::<"string", u16>::from_request(&req).is_err());
        assert!(QueryParam::<"string", u32>::from_request(&req).is_err());
        assert!(QueryParam::<"string", u64>::from_request(&req).is_err());
        assert!(QueryParam::<"string", u128>::from_request(&req).is_err());
        assert!(QueryParam::<"string", usize>::from_request(&req).is_err());
        assert!(matches!(
            QueryParam::<"string", String>::from_request(&req).map(QueryParam::into_inner),
            Ok(value) if value == "Rust"
        ));
        assert!(
            matches!(QueryParam::<"string", TestCustom>::from_request(&req).map(QueryParam::into_inner), Ok(value) if value.0 == "RUST")
        );

        assert!(QueryParam::<"custom", bool>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", char>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", f32>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", f64>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", i8>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", i16>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", i32>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", i64>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", i128>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", isize>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", u8>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", u16>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", u32>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", u64>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", u128>::from_request(&req).is_err());
        assert!(QueryParam::<"custom", usize>::from_request(&req).is_err());
        assert!(matches!(
            QueryParam::<"custom", String>::from_request(&req).map(QueryParam::into_inner),
            Ok(value) if value == "hello"
        ));
        assert!(
            matches!(QueryParam::<"custom", TestCustom>::from_request(&req).map(QueryParam::into_inner), Ok(value) if value.0 == "HELLO")
        );

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", query_param_parse))
        .start()
        .await
        .unwrap();

    let res = server
        .get(
            "/test?bool=true&char=c&int=1729&float=1.618&string=Rust&custom=hello",
            |req| req,
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_header_parsing() {
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestCustom(String);

    impl FromStr for TestCustom {
        type Err = Infallible;

        fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
            Ok(Self(s.to_uppercase()))
        }
    }

    async fn header_parse(req: Request) -> Response {
        assert!(matches!(
            Header::<"bool", bool>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![true]
        ));
        assert!(Header::<"bool", char>::from_request(&req).is_err());
        assert!(Header::<"bool", f32>::from_request(&req).is_err());
        assert!(Header::<"bool", f64>::from_request(&req).is_err());
        assert!(Header::<"bool", i8>::from_request(&req).is_err());
        assert!(Header::<"bool", i16>::from_request(&req).is_err());
        assert!(Header::<"bool", i32>::from_request(&req).is_err());
        assert!(Header::<"bool", i64>::from_request(&req).is_err());
        assert!(Header::<"bool", i128>::from_request(&req).is_err());
        assert!(Header::<"bool", isize>::from_request(&req).is_err());
        assert!(Header::<"bool", u8>::from_request(&req).is_err());
        assert!(Header::<"bool", u16>::from_request(&req).is_err());
        assert!(Header::<"bool", u32>::from_request(&req).is_err());
        assert!(Header::<"bool", u64>::from_request(&req).is_err());
        assert!(Header::<"bool", u128>::from_request(&req).is_err());
        assert!(Header::<"bool", usize>::from_request(&req).is_err());
        assert!(matches!(
            Header::<"bool", String>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec!["true"]
        ));
        assert!(
            matches!(Header::<"bool", TestCustom>::from_request(&req).map(Header::into_inner), Ok(value) if value == vec![TestCustom("TRUE".to_owned())])
        );

        assert!(Header::<"char", bool>::from_request(&req).is_err());
        assert!(matches!(
            Header::<"char", char>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec!['c']
        ));
        assert!(Header::<"char", f32>::from_request(&req).is_err());
        assert!(Header::<"char", f64>::from_request(&req).is_err());
        assert!(Header::<"char", i8>::from_request(&req).is_err());
        assert!(Header::<"char", i16>::from_request(&req).is_err());
        assert!(Header::<"char", i32>::from_request(&req).is_err());
        assert!(Header::<"char", i64>::from_request(&req).is_err());
        assert!(Header::<"char", i128>::from_request(&req).is_err());
        assert!(Header::<"char", isize>::from_request(&req).is_err());
        assert!(Header::<"char", u8>::from_request(&req).is_err());
        assert!(Header::<"char", u16>::from_request(&req).is_err());
        assert!(Header::<"char", u32>::from_request(&req).is_err());
        assert!(Header::<"char", u64>::from_request(&req).is_err());
        assert!(Header::<"char", u128>::from_request(&req).is_err());
        assert!(Header::<"char", usize>::from_request(&req).is_err());
        assert!(matches!(
            Header::<"char", String>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec!["c"]
        ));
        assert!(
            matches!(Header::<"char", TestCustom>::from_request(&req).map(Header::into_inner), Ok(value) if value == vec![TestCustom("C".to_owned())])
        );

        assert!(Header::<"int", bool>::from_request(&req).is_err());
        assert!(Header::<"int", char>::from_request(&req).is_err());
        assert!(matches!(
            Header::<"int", f32>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1729.0]
        ));
        assert!(matches!(
            Header::<"int", f64>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1729.0]
        ));
        assert!(Header::<"int", i8>::from_request(&req).is_err());
        assert!(matches!(
            Header::<"int", i16>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1729]
        ));
        assert!(matches!(
            Header::<"int", i32>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1729]
        ));
        assert!(matches!(
            Header::<"int", i64>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1729]
        ));
        assert!(matches!(
            Header::<"int", i128>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1729]
        ));
        assert!(matches!(
            Header::<"int", isize>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1729]
        ));
        assert!(Header::<"int", u8>::from_request(&req).is_err());
        assert!(matches!(
            Header::<"int", u16>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1729]
        ));
        assert!(matches!(
            Header::<"int", u32>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1729]
        ));
        assert!(matches!(
            Header::<"int", u64>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1729]
        ));
        assert!(matches!(
            Header::<"int", u128>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1729]
        ));
        assert!(matches!(
            Header::<"int", usize>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1729]
        ));
        assert!(matches!(
            Header::<"int", String>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec!["1729".to_owned()]
        ));
        assert!(
            matches!(Header::<"int", TestCustom>::from_request(&req).map(Header::into_inner), Ok(value) if value == vec![TestCustom("1729".to_owned())])
        );

        assert!(Header::<"float", bool>::from_request(&req).is_err());
        assert!(Header::<"float", char>::from_request(&req).is_err());
        assert!(matches!(
            Header::<"float", f32>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1.618]
        ));
        assert!(matches!(
            Header::<"float", f64>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec![1.618]
        ));
        assert!(Header::<"float", i8>::from_request(&req).is_err());
        assert!(Header::<"float", i16>::from_request(&req).is_err());
        assert!(Header::<"float", i32>::from_request(&req).is_err());
        assert!(Header::<"float", i64>::from_request(&req).is_err());
        assert!(Header::<"float", i128>::from_request(&req).is_err());
        assert!(Header::<"float", isize>::from_request(&req).is_err());
        assert!(Header::<"float", u8>::from_request(&req).is_err());
        assert!(Header::<"float", u16>::from_request(&req).is_err());
        assert!(Header::<"float", u32>::from_request(&req).is_err());
        assert!(Header::<"float", u64>::from_request(&req).is_err());
        assert!(Header::<"float", u128>::from_request(&req).is_err());
        assert!(Header::<"float", usize>::from_request(&req).is_err());
        assert!(matches!(
            Header::<"float", String>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec!["1.618".to_owned()]
        ));
        assert!(
            matches!(Header::<"float", TestCustom>::from_request(&req).map(Header::into_inner), Ok(value) if value == vec![TestCustom("1.618".to_owned())])
        );

        assert!(Header::<"string", bool>::from_request(&req).is_err());
        assert!(Header::<"string", char>::from_request(&req).is_err());
        assert!(Header::<"string", f32>::from_request(&req).is_err());
        assert!(Header::<"string", f64>::from_request(&req).is_err());
        assert!(Header::<"string", i8>::from_request(&req).is_err());
        assert!(Header::<"string", i16>::from_request(&req).is_err());
        assert!(Header::<"string", i32>::from_request(&req).is_err());
        assert!(Header::<"string", i64>::from_request(&req).is_err());
        assert!(Header::<"string", i128>::from_request(&req).is_err());
        assert!(Header::<"string", isize>::from_request(&req).is_err());
        assert!(Header::<"string", u8>::from_request(&req).is_err());
        assert!(Header::<"string", u16>::from_request(&req).is_err());
        assert!(Header::<"string", u32>::from_request(&req).is_err());
        assert!(Header::<"string", u64>::from_request(&req).is_err());
        assert!(Header::<"string", u128>::from_request(&req).is_err());
        assert!(Header::<"string", usize>::from_request(&req).is_err());
        assert!(matches!(
            Header::<"string", String>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec!["Rust".to_owned()]
        ));
        assert!(
            matches!(Header::<"string", TestCustom>::from_request(&req).map(Header::into_inner), Ok(value) if value == vec![TestCustom("RUST".to_owned())])
        );

        assert!(Header::<"custom", bool>::from_request(&req).is_err());
        assert!(Header::<"custom", char>::from_request(&req).is_err());
        assert!(Header::<"custom", f32>::from_request(&req).is_err());
        assert!(Header::<"custom", f64>::from_request(&req).is_err());
        assert!(Header::<"custom", i8>::from_request(&req).is_err());
        assert!(Header::<"custom", i16>::from_request(&req).is_err());
        assert!(Header::<"custom", i32>::from_request(&req).is_err());
        assert!(Header::<"custom", i64>::from_request(&req).is_err());
        assert!(Header::<"custom", i128>::from_request(&req).is_err());
        assert!(Header::<"custom", isize>::from_request(&req).is_err());
        assert!(Header::<"custom", u8>::from_request(&req).is_err());
        assert!(Header::<"custom", u16>::from_request(&req).is_err());
        assert!(Header::<"custom", u32>::from_request(&req).is_err());
        assert!(Header::<"custom", u64>::from_request(&req).is_err());
        assert!(Header::<"custom", u128>::from_request(&req).is_err());
        assert!(Header::<"custom", usize>::from_request(&req).is_err());
        assert!(matches!(
            Header::<"custom", String>::from_request(&req).map(Header::into_inner),
            Ok(value) if value == vec!["hello".to_owned()]
        ));
        assert!(
            matches!(Header::<"custom", TestCustom>::from_request(&req).map(Header::into_inner), Ok(value) if value == vec![TestCustom("HELLO".to_owned())])
        );

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", header_parse))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| {
            req.header("bool", "true")
                .header("char", "c")
                .header("int", "1729")
                .header("float", "1.618")
                .header("string", "Rust")
                .header("custom", "hello")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_cookie_parsing() {
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestCustom(String);

    impl FromStr for TestCustom {
        type Err = Infallible;

        fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
            Ok(Self(s.to_uppercase()))
        }
    }

    async fn cookie_parse(req: Request) -> Response {
        assert!(matches!(
            Cookie::<"bool", bool>::from_request(&req).map(Cookie::into_inner),
            Ok(true)
        ));
        assert!(Cookie::<"bool", char>::from_request(&req).is_err());
        assert!(Cookie::<"bool", f32>::from_request(&req).is_err());
        assert!(Cookie::<"bool", f64>::from_request(&req).is_err());
        assert!(Cookie::<"bool", i8>::from_request(&req).is_err());
        assert!(Cookie::<"bool", i16>::from_request(&req).is_err());
        assert!(Cookie::<"bool", i32>::from_request(&req).is_err());
        assert!(Cookie::<"bool", i64>::from_request(&req).is_err());
        assert!(Cookie::<"bool", i128>::from_request(&req).is_err());
        assert!(Cookie::<"bool", isize>::from_request(&req).is_err());
        assert!(Cookie::<"bool", u8>::from_request(&req).is_err());
        assert!(Cookie::<"bool", u16>::from_request(&req).is_err());
        assert!(Cookie::<"bool", u32>::from_request(&req).is_err());
        assert!(Cookie::<"bool", u64>::from_request(&req).is_err());
        assert!(Cookie::<"bool", u128>::from_request(&req).is_err());
        assert!(Cookie::<"bool", usize>::from_request(&req).is_err());
        assert!(matches!(
            Cookie::<"bool", String>::from_request(&req).map(Cookie::into_inner),
            Ok(value) if value == "true"
        ));
        assert!(
            matches!(Cookie::<"bool", TestCustom>::from_request(&req).map(Cookie::into_inner), Ok(value) if value.0 == "TRUE")
        );

        assert!(Cookie::<"char", bool>::from_request(&req).is_err());
        assert!(matches!(
            Cookie::<"char", char>::from_request(&req).map(Cookie::into_inner),
            Ok('c')
        ));
        assert!(Cookie::<"char", f32>::from_request(&req).is_err());
        assert!(Cookie::<"char", f64>::from_request(&req).is_err());
        assert!(Cookie::<"char", i8>::from_request(&req).is_err());
        assert!(Cookie::<"char", i16>::from_request(&req).is_err());
        assert!(Cookie::<"char", i32>::from_request(&req).is_err());
        assert!(Cookie::<"char", i64>::from_request(&req).is_err());
        assert!(Cookie::<"char", i128>::from_request(&req).is_err());
        assert!(Cookie::<"char", isize>::from_request(&req).is_err());
        assert!(Cookie::<"char", u8>::from_request(&req).is_err());
        assert!(Cookie::<"char", u16>::from_request(&req).is_err());
        assert!(Cookie::<"char", u32>::from_request(&req).is_err());
        assert!(Cookie::<"char", u64>::from_request(&req).is_err());
        assert!(Cookie::<"char", u128>::from_request(&req).is_err());
        assert!(Cookie::<"char", usize>::from_request(&req).is_err());
        assert!(matches!(
            Cookie::<"char", String>::from_request(&req).map(Cookie::into_inner),
            Ok(value) if value == "c"
        ));
        assert!(
            matches!(Cookie::<"char", TestCustom>::from_request(&req).map(Cookie::into_inner), Ok(value) if value.0 == "C")
        );

        assert!(Cookie::<"int", bool>::from_request(&req).is_err());
        assert!(Cookie::<"int", char>::from_request(&req).is_err());
        assert!(matches!(
            Cookie::<"int", f32>::from_request(&req).map(Cookie::into_inner),
            Ok(1729.0)
        ));
        assert!(matches!(
            Cookie::<"int", f64>::from_request(&req).map(Cookie::into_inner),
            Ok(1729.0)
        ));
        assert!(Cookie::<"int", i8>::from_request(&req).is_err());
        assert!(matches!(
            Cookie::<"int", i16>::from_request(&req).map(Cookie::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            Cookie::<"int", i32>::from_request(&req).map(Cookie::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            Cookie::<"int", i64>::from_request(&req).map(Cookie::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            Cookie::<"int", i128>::from_request(&req).map(Cookie::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            Cookie::<"int", isize>::from_request(&req).map(Cookie::into_inner),
            Ok(1729)
        ));
        assert!(Cookie::<"int", u8>::from_request(&req).is_err());
        assert!(matches!(
            Cookie::<"int", u16>::from_request(&req).map(Cookie::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            Cookie::<"int", u32>::from_request(&req).map(Cookie::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            Cookie::<"int", u64>::from_request(&req).map(Cookie::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            Cookie::<"int", u128>::from_request(&req).map(Cookie::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            Cookie::<"int", usize>::from_request(&req).map(Cookie::into_inner),
            Ok(1729)
        ));
        assert!(matches!(
            Cookie::<"int", String>::from_request(&req).map(Cookie::into_inner),
            Ok(value) if value == "1729"
        ));
        assert!(
            matches!(Cookie::<"int", TestCustom>::from_request(&req).map(Cookie::into_inner), Ok(value) if value.0 == "1729")
        );

        assert!(Cookie::<"float", bool>::from_request(&req).is_err());
        assert!(Cookie::<"float", char>::from_request(&req).is_err());
        assert!(matches!(
            Cookie::<"float", f32>::from_request(&req).map(Cookie::into_inner),
            Ok(1.618)
        ));
        assert!(matches!(
            Cookie::<"float", f64>::from_request(&req).map(Cookie::into_inner),
            Ok(1.618)
        ));
        assert!(Cookie::<"float", i8>::from_request(&req).is_err());
        assert!(Cookie::<"float", i16>::from_request(&req).is_err());
        assert!(Cookie::<"float", i32>::from_request(&req).is_err());
        assert!(Cookie::<"float", i64>::from_request(&req).is_err());
        assert!(Cookie::<"float", i128>::from_request(&req).is_err());
        assert!(Cookie::<"float", isize>::from_request(&req).is_err());
        assert!(Cookie::<"float", u8>::from_request(&req).is_err());
        assert!(Cookie::<"float", u16>::from_request(&req).is_err());
        assert!(Cookie::<"float", u32>::from_request(&req).is_err());
        assert!(Cookie::<"float", u64>::from_request(&req).is_err());
        assert!(Cookie::<"float", u128>::from_request(&req).is_err());
        assert!(Cookie::<"float", usize>::from_request(&req).is_err());
        assert!(matches!(
            Cookie::<"float", String>::from_request(&req).map(Cookie::into_inner),
            Ok(value) if value == "1.618"
        ));
        assert!(
            matches!(Cookie::<"float", TestCustom>::from_request(&req).map(Cookie::into_inner), Ok(value) if value.0 == "1.618")
        );

        assert!(Cookie::<"string", bool>::from_request(&req).is_err());
        assert!(Cookie::<"string", char>::from_request(&req).is_err());
        assert!(Cookie::<"string", f32>::from_request(&req).is_err());
        assert!(Cookie::<"string", f64>::from_request(&req).is_err());
        assert!(Cookie::<"string", i8>::from_request(&req).is_err());
        assert!(Cookie::<"string", i16>::from_request(&req).is_err());
        assert!(Cookie::<"string", i32>::from_request(&req).is_err());
        assert!(Cookie::<"string", i64>::from_request(&req).is_err());
        assert!(Cookie::<"string", i128>::from_request(&req).is_err());
        assert!(Cookie::<"string", isize>::from_request(&req).is_err());
        assert!(Cookie::<"string", u8>::from_request(&req).is_err());
        assert!(Cookie::<"string", u16>::from_request(&req).is_err());
        assert!(Cookie::<"string", u32>::from_request(&req).is_err());
        assert!(Cookie::<"string", u64>::from_request(&req).is_err());
        assert!(Cookie::<"string", u128>::from_request(&req).is_err());
        assert!(Cookie::<"string", usize>::from_request(&req).is_err());
        assert!(matches!(
            Cookie::<"string", String>::from_request(&req).map(Cookie::into_inner),
            Ok(value) if value == "Rust"
        ));
        assert!(
            matches!(Cookie::<"string", TestCustom>::from_request(&req).map(Cookie::into_inner), Ok(value) if value.0 == "RUST")
        );

        assert!(Cookie::<"custom", bool>::from_request(&req).is_err());
        assert!(Cookie::<"custom", char>::from_request(&req).is_err());
        assert!(Cookie::<"custom", f32>::from_request(&req).is_err());
        assert!(Cookie::<"custom", f64>::from_request(&req).is_err());
        assert!(Cookie::<"custom", i8>::from_request(&req).is_err());
        assert!(Cookie::<"custom", i16>::from_request(&req).is_err());
        assert!(Cookie::<"custom", i32>::from_request(&req).is_err());
        assert!(Cookie::<"custom", i64>::from_request(&req).is_err());
        assert!(Cookie::<"custom", i128>::from_request(&req).is_err());
        assert!(Cookie::<"custom", isize>::from_request(&req).is_err());
        assert!(Cookie::<"custom", u8>::from_request(&req).is_err());
        assert!(Cookie::<"custom", u16>::from_request(&req).is_err());
        assert!(Cookie::<"custom", u32>::from_request(&req).is_err());
        assert!(Cookie::<"custom", u64>::from_request(&req).is_err());
        assert!(Cookie::<"custom", u128>::from_request(&req).is_err());
        assert!(Cookie::<"custom", usize>::from_request(&req).is_err());
        assert!(matches!(
            Cookie::<"custom", String>::from_request(&req).map(Cookie::into_inner),
            Ok(value) if value == "hello"
        ));
        assert!(
            matches!(Cookie::<"custom", TestCustom>::from_request(&req).map(Cookie::into_inner), Ok(value) if value.0 == "HELLO")
        );

        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", cookie_parse))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| {
            req.header(
                http::header::COOKIE,
                "bool=true; char=c; int=1729; float=1.618; string=Rust; custom=hello",
            )
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_repeated_header() {
    #[handler]
    async fn repeated_header(headers: HeaderMap) {
        let test_header = headers.get("Test-Header").unwrap();
        assert_eq!(test_header, &["foo", "bar", "baz"]);
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", repeated_header))
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| {
            req.header("Test-Header", "foo")
                .header("Test-Header", "bar")
                .header("Test-Header", "baz")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_new() {
    #[handler]
    async fn response_new() -> Response {
        Response::new()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_new))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.bytes().await.unwrap().is_empty());

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_new_error() {
    #[handler]
    async fn response_new_error() -> Response {
        Response::new_error(Error::UnsupportedMediaType)
            .status_code(StatusCode::IM_A_TEAPOT)
            .body("This body will not be set")
            .header("Test-Header", "This header will also not be set")
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_new_error))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert!(res.headers().get("Test-Header").is_none());
    assert_eq!(res.text().await.unwrap(), "{\"error\":\"the request body content does not match the `Content-Type` header, or the header is not present\"}");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_status_code() {
    #[handler]
    async fn response_status_code() -> Response {
        Response::new()
            .status_code(StatusCode::SEE_OTHER)
            .status_code(StatusCode::UNPROCESSABLE_ENTITY)
            .status_code(StatusCode::IM_A_TEAPOT)
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_status_code))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::IM_A_TEAPOT);
    assert!(res.bytes().await.unwrap().is_empty());

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_body() {
    #[handler]
    async fn response_body() -> Response {
        Response::new().body("Hello, response body!")
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_body))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, response body!");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_body_or() {
    #[handler]
    async fn response_body_or() -> Response {
        Response::new()
            .body_or("first body")
            .body_or("second body")
            .body_or("third body")
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_body_or))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "first body");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_body_json() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    struct TestJson {
        num: i32,
    }

    #[handler]
    async fn response_body_json() -> Response {
        Response::new().body_json(TestJson { num: 123 })
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_body_json))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.json::<TestJson>().await.unwrap(), TestJson { num: 123 });

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_header() {
    #[handler]
    async fn response_header() -> Response {
        Response::new().header("Test-Header", "Hello, response header!")
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_header))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers().get("Test-Header").unwrap(),
        "Hello, response header!"
    );

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_cookie() {
    #[handler]
    async fn response_cookie() -> Response {
        Response::new()
            .cookie(SetCookie::new("test_cookie_1", "Hello, response cookie!").http_only(true))
            .cookie(
                SetCookie::new("test_cookie_2", "Goodbye, response cookie!")
                    .expire_after(Duration::from_secs(35792)),
            )
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_cookie))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get_all(http::header::SET_COOKIE)
            .into_iter()
            .collect::<Vec<_>>(),
        &[
            "test_cookie_1=Hello, response cookie!; HttpOnly",
            "test_cookie_2=Goodbye, response cookie!; Max-Age=35792"
        ]
    );

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_and() {
    #[handler]
    async fn response_and() -> Response {
        Response::new()
            .and(StatusCode::NOT_IMPLEMENTED)
            .and(StatusCode::IM_A_TEAPOT)
            .and("This body will be overridden")
            .and("by this one")
            .and(Response::new().header("Test-Header", ": )"))
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_and))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::IM_A_TEAPOT);
    assert_eq!(res.headers().get("Test-Header").unwrap(), ": )");
    assert_eq!(res.text().await.unwrap(), "by this one");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_from_response() {
    async fn response_from_response(_: Request) -> Response {
        Response::new()
            .status_code(StatusCode::IM_A_TEAPOT)
            .into_response()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_from_response))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::IM_A_TEAPOT);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_from_str() {
    async fn response_from_str(_: Request) -> Response {
        "Hello, response str!".into_response()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_from_str))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, response str!");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_from_string() {
    async fn response_from_string(_: Request) -> Response {
        "Hello, response string!".to_owned().into_response()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_from_string))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, response string!");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_from_string_ref() {
    async fn response_from_string_ref(_: Request) -> Response {
        (&("Hello, response string ref!".to_owned())).into_response()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_from_string_ref))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, response string ref!");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_from_body_string() {
    async fn response_from_body_string(_: Request) -> Response {
        BodyString("Hello, response body string!".to_owned()).into_response()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_from_body_string))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, response body string!");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_from_json() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    struct TestJson {
        num: i32,
    }

    async fn response_from_json(_: Request) -> Response {
        Json(TestJson { num: 123 }).into_response()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_from_json))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.json::<TestJson>().await.unwrap(), TestJson { num: 123 });

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_from_result_ok() {
    async fn response_from_result_ok(_: Request) -> Response {
        Result::Ok("Hello, response result!").into_response()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_from_result_ok))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, response result!");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_from_result_err() {
    async fn response_from_result_err(_: Request) -> Response {
        Result::<()>::Err(Error::UnsupportedMediaType).into_response()
    }

    let server = TestServer::new()
        .config(|server| server.get("/test", response_from_result_err))
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(res.text().await.unwrap(), "{\"error\":\"the request body content does not match the `Content-Type` header, or the header is not present\"}");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_response_from_tuple() {
    async fn response_from_tuple0(_: Request) -> Response {
        ().into_response()
    }

    async fn response_from_tuple1(_: Request) -> Response {
        ("1",).into_response()
    }

    async fn response_from_tuple2(_: Request) -> Response {
        ("1", "2").into_response()
    }

    async fn response_from_tuple3(_: Request) -> Response {
        ("1", "2", "3").into_response()
    }

    async fn response_from_tuple4(_: Request) -> Response {
        ("1", "2", "3", "4").into_response()
    }

    async fn response_from_tuple5(_: Request) -> Response {
        ("1", "2", "3", "4", "5").into_response()
    }

    async fn response_from_tuple6(_: Request) -> Response {
        ("1", "2", "3", "4", "5", "6").into_response()
    }

    async fn response_from_tuple7(_: Request) -> Response {
        ("1", "2", "3", "4", "5", "6", "7").into_response()
    }

    async fn response_from_tuple8(_: Request) -> Response {
        ("1", "2", "3", "4", "5", "6", "7", "8").into_response()
    }

    async fn response_from_tuple9(_: Request) -> Response {
        ("1", "2", "3", "4", "5", "6", "7", "8", "9").into_response()
    }

    async fn response_from_tuple10(_: Request) -> Response {
        ("1", "2", "3", "4", "5", "6", "7", "8", "9", "10").into_response()
    }

    async fn response_from_tuple11(_: Request) -> Response {
        ("1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11").into_response()
    }

    async fn response_from_tuple12(_: Request) -> Response {
        (
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12",
        )
            .into_response()
    }

    async fn response_from_tuple13(_: Request) -> Response {
        (
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13",
        )
            .into_response()
    }

    async fn response_from_tuple14(_: Request) -> Response {
        (
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14",
        )
            .into_response()
    }

    async fn response_from_tuple15(_: Request) -> Response {
        (
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
        )
            .into_response()
    }

    async fn response_from_tuple16(_: Request) -> Response {
        (
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16",
        )
            .into_response()
    }

    let server = TestServer::new()
        .config(|server| {
            server
                .get("/test/0", response_from_tuple0)
                .get("/test/1", response_from_tuple1)
                .get("/test/2", response_from_tuple2)
                .get("/test/3", response_from_tuple3)
                .get("/test/4", response_from_tuple4)
                .get("/test/5", response_from_tuple5)
                .get("/test/6", response_from_tuple6)
                .get("/test/7", response_from_tuple7)
                .get("/test/8", response_from_tuple8)
                .get("/test/9", response_from_tuple9)
                .get("/test/10", response_from_tuple10)
                .get("/test/11", response_from_tuple11)
                .get("/test/12", response_from_tuple12)
                .get("/test/13", response_from_tuple13)
                .get("/test/14", response_from_tuple14)
                .get("/test/15", response_from_tuple15)
                .get("/test/16", response_from_tuple16)
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test/0", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res.text().await.unwrap().is_empty());

    for i in 1..=16 {
        let res = server
            .get(&format!("/test/{}", i), |req| req)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(res.text().await.unwrap(), i.to_string());
    }

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_http_methods() {
    #[handler]
    async fn handler_get() -> &'static str {
        "Hello, GET method!"
    }

    #[handler]
    async fn handler_head() -> StatusCode {
        StatusCode::IM_A_TEAPOT
    }

    #[handler]
    async fn handler_post() -> &'static str {
        "Hello, POST method!"
    }

    #[handler]
    async fn handler_put() -> &'static str {
        "Hello, PUT method!"
    }

    #[handler]
    async fn handler_delete() -> &'static str {
        "Hello, DELETE method!"
    }

    #[handler]
    async fn handler_connect() -> &'static str {
        "Hello, CONNECT method!"
    }

    #[handler]
    async fn handler_options() -> &'static str {
        "Hello, OPTIONS method!"
    }

    #[handler]
    async fn handler_trace() -> &'static str {
        "Hello, TRACE method!"
    }

    #[handler]
    async fn handler_patch() -> &'static str {
        "Hello, PATCH method!"
    }

    let server = TestServer::new()
        .config(|server| {
            server
                .get("/test", handler_get)
                .head("/test", handler_head)
                .post("/test", handler_post)
                .put("/test", handler_put)
                .delete("/test", handler_delete)
                .connect("/test", handler_connect)
                .options("/test", handler_options)
                .trace("/test", handler_trace)
                .patch("/test", handler_patch)
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, GET method!");

    let res = server.head("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::IM_A_TEAPOT);

    let res = server.post("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, POST method!");

    let res = server.put("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, PUT method!");

    let res = server.delete("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, DELETE method!");

    // CONNECT is not easily testable
    // let res = server.connect("/test", |req| req).await.unwrap();
    // assert_eq!(res.status(), StatusCode::OK);
    // assert_eq!(res.text().await.unwrap(), "Hello, CONNECT method!");

    let res = server.options("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, OPTIONS method!");

    let res = server.trace("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, TRACE method!");

    let res = server.patch("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, PATCH method!");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_middleware() {
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Token(String);

    impl FromStr for Token {
        type Err = Infallible;

        fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
            Ok(Self(s.to_owned()))
        }
    }

    #[middleware]
    async fn auth(req: Request, next: NextFn, token: CookieOptional<"token", Token>) -> Response {
        match &*token {
            Some(token) => {
                if token.0.as_str() == "0123456789ABCDEF" {
                    next.call(req).await
                } else {
                    Response::new().status_code(StatusCode::FORBIDDEN)
                }
            }
            None => Response::new().status_code(StatusCode::UNAUTHORIZED),
        }
    }

    #[handler]
    async fn secret() -> &'static str {
        "35792"
    }

    let server = TestServer::new()
        .config(|server| {
            server.route_group(
                RouteGroup::new("/admin")
                    .with_middleware(auth)
                    .get("/secret", secret),
            )
        })
        .start()
        .await
        .unwrap();

    let res = server
        .get("/admin/secret", |req| {
            req.header(http::header::COOKIE, "token=0123456789ABCDEF")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "35792");

    let res = server.get("/admin/secret", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    let res = server
        .get("/admin/secret", |req| {
            req.header(http::header::COOKIE, "token=invalid")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_middleware_mutate_response() {
    #[middleware]
    async fn test_header_middleware(req: Request, next: NextFn) -> Response {
        next.call(req).await.header("Test-Header", "35792")
    }

    #[handler]
    async fn empty_handler() {}

    let server = TestServer::new()
        .config(|server| {
            server
                .with_middleware(test_header_middleware)
                .get("/test", empty_handler)
        })
        .start()
        .await
        .unwrap();

    let res = server
        .get("/test", |req| {
            req.header(http::header::COOKIE, "token=0123456789ABCDEF")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers().get("Test-Header").unwrap(), "35792");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_middleware_prevent_handler() {
    #[middleware]
    async fn stop_middleware() -> Response {
        Response::new().status_code(StatusCode::NO_CONTENT)
    }

    #[handler]
    async fn unused_handler() -> &'static str {
        "this should not be included in the response"
    }

    let server = TestServer::new()
        .config(|server| {
            server
                .with_middleware(stop_middleware)
                .get("/test", unused_handler)
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert!(res.text().await.unwrap().is_empty());

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_middleware_prevent_middleware() {
    #[middleware]
    async fn middleware1() -> Response {
        Response::new().status_code(StatusCode::NO_CONTENT)
    }

    #[middleware]
    async fn middleware2() -> Response {
        Response::new().status_code(StatusCode::IM_A_TEAPOT)
    }

    #[handler]
    async fn unused_handler() -> &'static str {
        "this should not be included in the response"
    }

    let server = TestServer::new()
        .config(|server| {
            server
                .with_middleware(middleware1)
                .with_middleware(middleware2)
                .get("/test", unused_handler)
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert!(res.text().await.unwrap().is_empty());

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_middleware_multiple_layers() {
    #[middleware]
    async fn middleware1(
        req: Request,
        next: NextFn,
        do_next: QueryParamOptional<"mid1">,
    ) -> Response {
        if do_next.is_none() {
            next(req).await
        } else {
            Response::new().body("response from mid1")
        }
    }

    #[middleware]
    async fn middleware2(
        req: Request,
        next: NextFn,
        do_next: QueryParamOptional<"mid2">,
    ) -> Response {
        if do_next.is_none() {
            next(req).await
        } else {
            Response::new().body("response from mid2")
        }
    }

    #[middleware]
    async fn middleware3(
        req: Request,
        next: NextFn,
        do_next: QueryParamOptional<"mid3">,
    ) -> Response {
        if do_next.is_none() {
            next(req).await
        } else {
            Response::new().body("response from mid3")
        }
    }

    #[handler]
    async fn final_handler() -> &'static str {
        "response from handler"
    }

    let server = TestServer::new()
        .config(|server| {
            server
                .with_middleware(middleware1)
                .with_middleware(middleware2)
                .with_middleware(middleware3)
                .get("/test", final_handler)
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test?mid1=", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "response from mid1");

    let res = server.get("/test?mid2=", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "response from mid2");

    let res = server.get("/test?mid3=", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "response from mid3");

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "response from handler");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_middleware_order() {
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestOrder(Vec<usize>);

    #[middleware]
    async fn middleware1(req: Request, next: NextFn, local_state: LocalState) -> Response {
        local_state
            .with(|local_state| match local_state.get_mut::<TestOrder>() {
                Some(order) => {
                    order.0.push(1);
                }
                None => {
                    local_state.insert(TestOrder(vec![1]));
                }
            })
            .await;
        next(req).await.header("Pop-Order", "1")
    }

    #[middleware]
    async fn middleware2(req: Request, next: NextFn, local_state: LocalState) -> Response {
        local_state
            .with(|local_state| match local_state.get_mut::<TestOrder>() {
                Some(order) => {
                    order.0.push(2);
                }
                None => {
                    local_state.insert(TestOrder(vec![2]));
                }
            })
            .await;
        next(req).await.header("Pop-Order", "2")
    }

    #[middleware]
    async fn middleware3(req: Request, next: NextFn, local_state: LocalState) -> Response {
        local_state
            .with(|local_state| match local_state.get_mut::<TestOrder>() {
                Some(order) => {
                    order.0.push(3);
                }
                None => {
                    local_state.insert(TestOrder(vec![3]));
                }
            })
            .await;
        next(req).await.header("Pop-Order", "3")
    }

    #[middleware]
    async fn middleware4(req: Request, next: NextFn, local_state: LocalState) -> Response {
        local_state
            .with(|local_state| match local_state.get_mut::<TestOrder>() {
                Some(order) => {
                    order.0.push(4);
                }
                None => {
                    local_state.insert(TestOrder(vec![4]));
                }
            })
            .await;
        next(req).await.header("Pop-Order", "4")
    }

    #[middleware]
    async fn middleware5(req: Request, next: NextFn, local_state: LocalState) -> Response {
        local_state
            .with(|local_state| match local_state.get_mut::<TestOrder>() {
                Some(order) => {
                    order.0.push(5);
                }
                None => {
                    local_state.insert(TestOrder(vec![5]));
                }
            })
            .await;
        next(req).await.header("Pop-Order", "5")
    }

    #[handler]
    async fn final_handler(local_state: LocalState) -> Response {
        let order = local_state
            .with(|local_state| local_state.remove::<TestOrder>())
            .await
            .unwrap();
        order.0.into_iter().fold(Response::new(), |res, value| {
            res.header("Push-Order", &value.to_string())
        })
    }

    let server = TestServer::new()
        .config(|server| {
            server
                .with_middleware(middleware1)
                .with_middleware(middleware2)
                .with_middleware(middleware3)
                .with_middleware(middleware4)
                .with_middleware(middleware5)
                .get("/test", final_handler)
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get_all("Push-Order")
            .into_iter()
            .collect::<Vec<_>>(),
        vec!["1", "2", "3", "4", "5"]
    );
    assert_eq!(
        res.headers()
            .get_all("Pop-Order")
            .into_iter()
            .collect::<Vec<_>>(),
        vec!["5", "4", "3", "2", "1"]
    );

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_local_middleware() {
    #[middleware]
    async fn local_middleware(req: Request, next: NextFn) -> Response {
        next(req).await.header("Local-Middleware", "1")
    }

    #[middleware]
    async fn recursive_middleware(req: Request, next: NextFn) -> Response {
        next(req).await.header("Recursive-Middleware", "1")
    }

    #[handler]
    async fn parent_handler() -> &'static str {
        "parent"
    }

    #[handler]
    async fn child_handler() -> &'static str {
        "child"
    }

    let server = TestServer::new()
        .config(|server| {
            server.route_group(
                RouteGroup::new("/test")
                    .with_local_middleware(local_middleware)
                    .with_middleware(recursive_middleware)
                    .get("/", parent_handler)
                    .route_group(RouteGroup::new("/child").get("/", child_handler)),
            )
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers().get("Local-Middleware").unwrap(), "1");
    assert_eq!(res.headers().get("Recursive-Middleware").unwrap(), "1");
    assert_eq!(res.text().await.unwrap(), "parent");

    let res = server.get("/test/child", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers().get("Local-Middleware"), None);
    assert_eq!(res.headers().get("Recursive-Middleware").unwrap(), "1");
    assert_eq!(res.text().await.unwrap(), "child");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_local_middleware_order() {
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TestOrder(Vec<&'static str>);

    #[middleware]
    async fn local_middleware(req: Request, next: NextFn, local_state: LocalState) -> Response {
        local_state
            .with(|local_state| match local_state.get_mut::<TestOrder>() {
                Some(order) => {
                    order.0.push("local");
                }
                None => {
                    local_state.insert(TestOrder(vec!["local"]));
                }
            })
            .await;
        next(req).await.header("Pop-Order", "local")
    }

    #[middleware]
    async fn recursive_middleware(req: Request, next: NextFn, local_state: LocalState) -> Response {
        local_state
            .with(|local_state| match local_state.get_mut::<TestOrder>() {
                Some(order) => {
                    order.0.push("recursive");
                }
                None => {
                    local_state.insert(TestOrder(vec!["recursive"]));
                }
            })
            .await;
        next(req).await.header("Pop-Order", "recursive")
    }

    #[handler]
    async fn final_handler(local_state: LocalState) -> Response {
        let order = local_state
            .with(|local_state| local_state.remove::<TestOrder>())
            .await
            .unwrap();
        order.0.into_iter().fold(Response::new(), |res, value| {
            res.header("Push-Order", value)
        })
    }

    let server = TestServer::new()
        .config(|server| {
            server.route_group(
                RouteGroup::new("/")
                    .with_middleware(recursive_middleware)
                    .route_group(
                        RouteGroup::new("/test")
                            .with_local_middleware(local_middleware)
                            .get("/", final_handler),
                    ),
            )
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get_all("Push-Order")
            .into_iter()
            .collect::<Vec<_>>(),
        vec!["recursive", "local"]
    );
    assert_eq!(
        res.headers()
            .get_all("Pop-Order")
            .into_iter()
            .collect::<Vec<_>>(),
        vec!["local", "recursive"]
    );

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_adjacent_route_groups() {
    #[middleware]
    async fn local_parent_middleware(req: Request, next: NextFn) -> Response {
        next(req).await.header("Parent-Local", "1")
    }

    #[middleware]
    async fn recursive_parent_middleware(req: Request, next: NextFn) -> Response {
        next(req).await.header("Parent-Recursive", "1")
    }

    #[middleware]
    async fn child_middleware1(req: Request, next: NextFn) -> Response {
        next(req).await.header("Child-1", "1")
    }

    #[middleware]
    async fn child_middleware2(req: Request, next: NextFn) -> Response {
        next(req).await.header("Child-2", "1")
    }

    #[handler]
    async fn final_handler() {}

    let server = TestServer::new()
        .config(|server| {
            server.route_group(
                RouteGroup::new("/test")
                    .with_local_middleware(local_parent_middleware)
                    .with_middleware(recursive_parent_middleware)
                    .route_group(
                        RouteGroup::new("/")
                            .with_middleware(child_middleware1)
                            .get("/", final_handler),
                    )
                    .route_group(
                        RouteGroup::new("/")
                            .with_middleware(child_middleware2)
                            .post("/", final_handler),
                    ),
            )
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers().get("Parent-Local"), None);
    assert_eq!(res.headers().get("Parent-Recursive").unwrap(), "1");
    assert_eq!(res.headers().get("Child-1").unwrap(), "1");
    assert_eq!(res.headers().get("Child-2"), None);

    let res = server.post("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers().get("Parent-Local"), None);
    assert_eq!(res.headers().get("Parent-Recursive").unwrap(), "1");
    assert_eq!(res.headers().get("Child-1"), None);
    assert_eq!(res.headers().get("Child-2").unwrap(), "1");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_wildcard_path() {
    #[handler]
    async fn wildcard_handler() -> &'static str {
        "wildcard response"
    }

    let server = TestServer::new()
        .config(|server| server.get("/test/{foo}", wildcard_handler))
        .start()
        .await
        .unwrap();

    let res = server.get("/test/bar", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "wildcard response");

    let res = server.get("/test/123", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "wildcard response");

    let res = server.get("/test/123.45", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "wildcard response");

    let res = server.get("/test/true", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "wildcard response");

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        res.text().await.unwrap(),
        "{\"error\":\"the requested path could not be found\"}"
    );

    let res = server.get("/test/bar/baz", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        res.text().await.unwrap(),
        "{\"error\":\"the requested path could not be found\"}"
    );

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_wildcard_path_preference() {
    #[handler]
    async fn wildcard_handler() -> &'static str {
        "wildcard response"
    }

    #[handler]
    async fn static_handler() -> &'static str {
        "static response"
    }

    let server = TestServer::new()
        .config(|server| {
            server
                .get("/test/{foo}", wildcard_handler)
                .get("/test/bar", static_handler)
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test/foo", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "wildcard response");

    let res = server.get("/test/bar", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "static response");

    let res = server.get("/test/baz", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "wildcard response");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}

#[tokio::test]
async fn test_inline_handler() {
    let server = TestServer::new()
        .config(|server| {
            server.get("/test", |req: Request| async move {
                Response::new()
                    .header("Content-Type", "text/plain")
                    .body(&format!(
                        "Hello, {}!",
                        req.query_param("name").unwrap_or("<unknown>")
                    ))
            })
        })
        .start()
        .await
        .unwrap();

    let res = server.get("/test?name=Will", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, Will!");

    let res = server.get("/test?name=Graydon", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, Graydon!");

    let res = server.get("/test", |req| req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.text().await.unwrap(), "Hello, <unknown>!");

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
}
