//! HTTP server building types.

use crate::error::Error;
use crate::http::{HttpMethod, StatusCode};
use crate::request::ServerRequest;
use crate::response::{ErrorBody, ServerResponse};
use crate::routing::{Route, RouteGroup, RoutePath};
use crate::state::StateManager;
use crate::typemap::TypeMap;
use hyper::body::Incoming;
use hyper::service::Service;
use hyper::{Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::mpsc::{
    channel, unbounded_channel, Receiver, Sender, UnboundedReceiver, UnboundedSender,
};

/// The sending half of a server shutdown signal channel.
#[derive(Debug)]
pub struct ShutdownSender(Sender<()>);

impl ShutdownSender {
    /// Sends a shutdown signal to the server.
    pub async fn shutdown(self) -> bool {
        self.0.send(()).await.is_ok()
    }
}

/// The receiving half of a server shutdown signal channel.
#[derive(Debug)]
pub struct ShutdownReceiver(Receiver<()>);

impl ShutdownReceiver {
    /// Waits to receive the shutdown signal.
    pub async fn await_signal(&mut self) -> bool {
        self.0.recv().await.is_some()
    }
}

/// Creates a server shutdown signal channel.
pub fn shutdown_signal() -> (ShutdownSender, ShutdownReceiver) {
    let (tx, rx) = channel(1);
    (ShutdownSender(tx), ShutdownReceiver(rx))
}

/// The sending half of an error reporting channel.
#[derive(Debug, Clone)]
pub struct ErrorSender(UnboundedSender<Arc<Error>>);

impl ErrorSender {
    /// Sends an error through the error reporting channel.
    pub fn report(&self, err: Arc<Error>) {
        _ = self.0.send(err);
    }
}

/// The receiving half of an error reporting channel.
#[derive(Debug)]
pub struct ErrorReceiver(UnboundedReceiver<Arc<Error>>);

impl ErrorReceiver {
    /// Waits to receive the next error from the server.
    pub async fn next(&mut self) -> Option<Arc<Error>> {
        self.0.recv().await
    }
}

/// Creates an error reporting stream. Note that the underlying channel is
/// unbounded, so errors must be consumed faster than they are produced to avoid
/// causing the process to run out of memory.
pub fn error_report_stream() -> (ErrorSender, ErrorReceiver) {
    let (tx, rx) = unbounded_channel();
    (ErrorSender(tx), ErrorReceiver(rx))
}

/// The internal server service managed by the `hyper` runtime.
struct ServerService {
    /// The collection of all registered routes.
    routes: Arc<HashMap<(HttpMethod, RoutePath), Route>>,
    /// The global application state management system.
    state: StateManager,
    /// The error reporting sender, if one was configured.
    error_sender: Option<ErrorSender>,
}

impl Service<Request<Incoming>> for ServerService {
    type Response = Response<String>;
    type Error = Error;
    type Future =
        Pin<Box<dyn Future<Output = std::result::Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let method = HttpMethod::from(req.method());
        let path = RoutePath::from(req.uri().path());
        let route = self.routes.get(&(method, path)).cloned();
        let state = self.state.clone();
        let error_sender = self.error_sender.clone();

        Box::pin(async move {
            let req = ServerRequest::new(req, state).await?;

            Ok(match route {
                Some(route) => {
                    let res = route.call(req).await;

                    if let ServerResponse::Err(err) = &res {
                        if err.source().is_server() {
                            if let Some(error_sender) = error_sender {
                                error_sender.report(Arc::clone(err));
                            }
                        }
                    }

                    res
                }
                None => ServerResponse::new()
                    .status_code(StatusCode::NotFound)
                    .body_json(ErrorBody::new("Not found")),
            }
            .into())
        })
    }
}

/// A web server. This is the core type used to configure and start a web
/// server.
#[derive(Default)]
pub struct Server {
    /// The collection of all registered routes.
    routes: HashMap<(HttpMethod, RoutePath), Route>,
    /// The global application state management system type map.
    state: TypeMap,
    /// The optional shutdown signal receiver.
    shutdown_receiver: Option<ShutdownReceiver>,
    /// The optional error reporting sender.
    error_sender: Option<ErrorSender>,
}

impl Server {
    /// Creates a new web server.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a route within the server.
    pub fn route<P, R>(mut self, method: HttpMethod, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<Route>,
    {
        self.routes.insert((method, path.into()), route.into());
        self
    }

    /// Registers a group of routes.
    pub fn route_group<P>(mut self, path: P, route_group: RouteGroup) -> Self
    where
        P: Into<RoutePath>,
    {
        let path = path.into();
        route_group
            .into_iter()
            .for_each(|((method, inner_path), route)| {
                self.routes.insert((method, path.join(inner_path)), route);
            });
        self
    }

    /// Shorthand for `.route(HttpMethod::Get, ...)`.
    pub fn get<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<Route>,
    {
        self.route(HttpMethod::Get, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Head, ...)`.
    pub fn head<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<Route>,
    {
        self.route(HttpMethod::Head, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Post, ...)`.
    pub fn post<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<Route>,
    {
        self.route(HttpMethod::Post, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Put, ...)`.
    pub fn put<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<Route>,
    {
        self.route(HttpMethod::Put, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Delete, ...)`.
    pub fn delete<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<Route>,
    {
        self.route(HttpMethod::Delete, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Connect, ...)`.
    pub fn connect<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<Route>,
    {
        self.route(HttpMethod::Connect, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Options, ...)`.
    pub fn options<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<Route>,
    {
        self.route(HttpMethod::Options, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Trace, ...)`.
    pub fn trace<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<Route>,
    {
        self.route(HttpMethod::Trace, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Patch, ...)`.
    pub fn patch<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<Route>,
    {
        self.route(HttpMethod::Patch, path, route)
    }

    /// Configures a value to be globally accessible within the state management
    /// system when the server runs. The value must implement `Clone`, so
    /// usually you'll want to wrap your data in an `Arc`. For interior
    /// mutability, use a `Mutex` or `RwLock` inside of the `Arc`.
    ///
    /// This can be called as many times as needed, so storing multiple states
    /// is possible. All values are stored in a type map, so values of the same
    /// type cannot be used.
    pub fn with_state<S>(mut self, state: S) -> Self
    where
        S: Clone + Send + Sync + 'static,
    {
        self.state.insert(state);
        self
    }

    /// Configures a shutdown signal to enable a graceful server shutdown. See
    /// [`shutdown_signal`] for more information.
    pub fn with_graceful_shutdown(mut self, shutdown_receiver: ShutdownReceiver) -> Self {
        self.shutdown_receiver = Some(shutdown_receiver);
        self
    }

    /// Configures an error reporting stream to handle errors occurring from
    /// within route handlers. See [`error_report_stream`] for more information.
    pub fn with_error_reporting(mut self, error_sender: ErrorSender) -> Self {
        self.error_sender = Some(error_sender);
        self
    }

    /// Starts the server running on the given address.
    pub async fn serve<A>(self, addr: A) -> io::Result<()>
    where
        A: ToSocketAddrs,
    {
        let routes = Arc::new(self.routes);
        let state = Arc::new(self.state);
        let listener = TcpListener::bind(addr).await?;
        let mut shutdown_receiver = self
            .shutdown_receiver
            .unwrap_or_else(|| shutdown_signal().1);

        loop {
            let conn = tokio::select! {
                conn = listener.accept() => {
                    match conn {
                        Ok((conn, _)) => conn,
                        Err(_) => continue,
                    }
                }
                shutdown = shutdown_receiver.await_signal() => {
                    if shutdown {
                        break;
                    } else {
                        continue;
                    }
                }
            };

            let conn = TokioIo::new(conn);

            let hyper_service = ServerService {
                routes: Arc::clone(&routes),
                state: StateManager(Arc::clone(&state)),
                error_sender: self.error_sender.clone(),
            };

            tokio::spawn(async move {
                _ = Builder::new(TokioExecutor::new())
                    .serve_connection(conn, hyper_service)
                    .await;
            });
        }

        Ok(())
    }
}
