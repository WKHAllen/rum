//! HTTP server building types.

use crate::error::Error;
use crate::http::Method;
use crate::middleware::Middleware;
use crate::request::Request;
use crate::response::Response;
use crate::routing::{RouteGroup, RouteHandler, RouteLevel, RoutePath};
use crate::state::StateManager;
use crate::typemap::TypeMap;
use hyper::body::Incoming;
use hyper::service::Service;
use hyper::{Request as HyperRequest, Response as HyperResponse};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
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
pub struct ErrorSender(UnboundedSender<Option<Arc<Error>>>);

impl ErrorSender {
    /// Sends an error through the error reporting channel.
    pub fn report(&self, err: Arc<Error>) {
        _ = self.0.send(Some(err));
    }

    /// Closes the error reporting channel.
    pub fn close(&self) {
        _ = self.0.send(None);
    }
}

/// The receiving half of an error reporting channel.
#[derive(Debug)]
pub enum ErrorReceiver {
    /// The server is running, and errors can still be received.
    Active(UnboundedReceiver<Option<Arc<Error>>>),
    /// The server has been closed.
    Done,
}

impl ErrorReceiver {
    /// Waits to receive the next error from the server.
    pub async fn next(&mut self) -> Option<Arc<Error>> {
        match self {
            Self::Active(receiver) => match receiver.recv().await.flatten() {
                Some(err) => Some(err),
                None => {
                    *self = Self::Done;
                    None
                }
            },
            Self::Done => None,
        }
    }
}

/// Creates an error reporting stream. Note that the underlying channel is
/// unbounded, so errors must be consumed faster than they are produced to avoid
/// causing the process to run out of memory.
pub fn error_report_stream() -> (ErrorSender, ErrorReceiver) {
    let (tx, rx) = unbounded_channel();
    (ErrorSender(tx), ErrorReceiver::Active(rx))
}

/// The internal server service managed by the `hyper` runtime.
struct ServerService {
    /// The collection of all registered routes.
    routes: Arc<RouteLevel>,
    /// The global application state management system.
    state: StateManager,
    /// The error reporting sender, if one was configured.
    error_sender: Option<ErrorSender>,
}

impl Service<HyperRequest<Incoming>> for ServerService {
    type Response = HyperResponse<String>;
    type Error = Error;
    type Future =
        Pin<Box<dyn Future<Output = std::result::Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: HyperRequest<Incoming>) -> Self::Future {
        let method = Method::from(req.method());
        let path = RoutePath::from(req.uri().path());
        let matched_path_and_route = self.routes.get(method, path);
        let state = self.state.clone();
        let error_sender = self.error_sender.clone();

        Box::pin(async move {
            Ok(match matched_path_and_route {
                Ok((matched_path, route)) => {
                    let req = Request::new(req, matched_path, state).await?;
                    let res = route.call(req).await;

                    if let Response::Err(err) = &res {
                        if err.source().is_server() {
                            if let Some(error_sender) = error_sender {
                                error_sender.report(Arc::clone(err));
                            }
                        }
                    }

                    res
                }
                Err(err) => err.as_response(),
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
    routes: RouteGroup,
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
    pub fn route<P, R>(mut self, method: Method, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.routes = self.routes.route(method, path, route);
        self
    }

    /// Registers a group of routes.
    pub fn route_group(mut self, route_group: RouteGroup) -> Self {
        self.routes = self.routes.route_group(route_group);
        self
    }

    /// Shorthand for `.route(Method::GET, ...)`.
    pub fn get<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(Method::GET, path, route)
    }

    /// Shorthand for `.route(Method::HEAD, ...)`.
    pub fn head<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(Method::HEAD, path, route)
    }

    /// Shorthand for `.route(Method::POST, ...)`.
    pub fn post<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(Method::POST, path, route)
    }

    /// Shorthand for `.route(Method::PUT, ...)`.
    pub fn put<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(Method::PUT, path, route)
    }

    /// Shorthand for `.route(Method::DELETE, ...)`.
    pub fn delete<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(Method::DELETE, path, route)
    }

    /// Shorthand for `.route(Method::CONNECT, ...)`.
    pub fn connect<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(Method::CONNECT, path, route)
    }

    /// Shorthand for `.route(Method::OPTIONS, ...)`.
    pub fn options<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(Method::OPTIONS, path, route)
    }

    /// Shorthand for `.route(Method::TRACE, ...)`.
    pub fn trace<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(Method::TRACE, path, route)
    }

    /// Shorthand for `.route(Method::PATCH, ...)`.
    pub fn patch<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(Method::PATCH, path, route)
    }

    /// Register middleware to be used on all routes at this level and all route
    /// groups below.
    pub fn with_middleware<M>(mut self, middleware: M) -> Self
    where
        M: Into<Middleware>,
    {
        self.routes.middleware.add_recursive(middleware.into());
        self
    }

    /// Registers middleware to be used on all routes at this level, but not on
    /// route groups below.
    pub fn with_local_middleware<M>(mut self, middleware: M) -> Self
    where
        M: Into<Middleware>,
    {
        self.routes.middleware.add_local(middleware.into());
        self
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
        let listener = TcpListener::bind(addr).await?;
        self.serve_with(listener).await;

        Ok(())
    }

    /// Starts the server running on the given TCP listener.
    pub async fn serve_with(self, listener: TcpListener) {
        let routes = Arc::new(self.routes.into_route_level());
        let state = Arc::new(self.state);
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

        if let Some(error_sender) = self.error_sender {
            error_sender.close();
        }
    }
}
