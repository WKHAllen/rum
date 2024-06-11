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
