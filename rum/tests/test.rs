use rum::error::Error;
use rum::prelude::*;
use rum::routing::{RoutePathMatchedSegment, RoutePathSegment};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
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
            req.header(hyper::header::CONTENT_TYPE, "text/plain")
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
            req.header(hyper::header::CONTENT_TYPE, "application/json")
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
            req.header(hyper::header::COOKIE, "parse_error=foo")
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
            req.header(hyper::header::CONTENT_TYPE, "application/json")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = server
        .get("/test/json", |req| {
            req.header(hyper::header::CONTENT_TYPE, "text/plain")
        })
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let errors = server.stop().await;
    assert_no_server_errors!(errors);
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
                .header(hyper::header::COOKIE, "num=567")
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
                .header(hyper::header::CONTENT_TYPE, "text/plain")
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
                .header(hyper::header::CONTENT_TYPE, "application/json")
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
        .get("/test", |req| req.header(hyper::header::COOKIE, "num=123"))
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
            req.header(hyper::header::COOKIE, "message=hello_cookies")
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
        .get("/test", |req| req.header(hyper::header::COOKIE, "num=123"))
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
        .get("/test", |req| req.header(hyper::header::COOKIE, "num=123"))
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
