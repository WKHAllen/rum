//! Types involving middleware.

use crate::request::Request;
use crate::response::Response;
use std::future::Future;
use std::pin::Pin;
use std::slice::{Iter, IterMut};
use std::sync::Arc;
use std::vec::IntoIter;

/// The next middleware function.
#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct NextFn(
    pub(crate) Arc<dyn Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>,
);

impl NextFn {
    /// Creates a `NextFn` from a middleware function.
    pub fn new<F, Fut>(f: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        Self(Arc::new(move |req| Box::pin(f(req))))
    }

    /// Calls the next middleware function.
    pub async fn call(&self, req: Request) -> Response {
        (self.0)(req).await
    }
}

#[cfg(feature = "nightly")]
impl FnOnce<(Request,)> for NextFn {
    type Output = Pin<Box<dyn Future<Output = Response> + Send>>;

    extern "rust-call" fn call_once(self, args: (Request,)) -> Self::Output {
        (self.0)(args.0)
    }
}

#[cfg(feature = "nightly")]
impl FnMut<(Request,)> for NextFn {
    extern "rust-call" fn call_mut(&mut self, args: (Request,)) -> Self::Output {
        (self.0)(args.0)
    }
}

#[cfg(feature = "nightly")]
impl Fn<(Request,)> for NextFn {
    extern "rust-call" fn call(&self, args: (Request,)) -> Self::Output {
        (self.0)(args.0)
    }
}

/// A middleware function.
#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct Middleware(
    pub(crate) Arc<dyn Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>,
);

impl Middleware {
    /// Creates middleware from the provided function.
    fn new<F, Fut>(middleware: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        Self(Arc::new(move |req| Box::pin(middleware(req))))
    }

    /// Calls the middleware.
    pub(crate) async fn call(&self, req: Request) -> Response {
        (self.0)(req).await
    }
}

impl<F, Fut> From<F> for Middleware
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    fn from(value: F) -> Self {
        Self::new(value)
    }
}

/// A middleware function, applied either to the local route group or
/// recursively to all sub-groups.
#[derive(Clone)]
pub(crate) enum AppliedMiddleware {
    /// A local route group.
    Local(Middleware),
    /// A recursive route group.
    Recursive(Middleware),
}

/// A collection of routing middleware.
#[derive(Clone, Default)]
pub(crate) struct MiddlewareCollection(pub(crate) Vec<AppliedMiddleware>);

impl MiddlewareCollection {
    /// Creates a new empty middleware collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a middleware function to the local route group.
    pub fn add_local(&mut self, middleware: Middleware) {
        self.0.push(AppliedMiddleware::Local(middleware));
    }

    /// Adds a middleware function to the local route group and all sub-groups.
    pub fn add_recursive(&mut self, middleware: Middleware) {
        self.0.push(AppliedMiddleware::Recursive(middleware));
    }
}

impl<'a> IntoIterator for &'a MiddlewareCollection {
    type Item = &'a AppliedMiddleware;
    type IntoIter = Iter<'a, AppliedMiddleware>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a mut MiddlewareCollection {
    type Item = &'a mut AppliedMiddleware;
    type IntoIter = IterMut<'a, AppliedMiddleware>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl IntoIterator for MiddlewareCollection {
    type Item = AppliedMiddleware;
    type IntoIter = IntoIter<AppliedMiddleware>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
