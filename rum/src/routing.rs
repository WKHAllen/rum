//! Types involving request routing.

use crate::error::{Error, Result};
use crate::http::HttpMethod;
use crate::middleware::{AppliedMiddleware, Middleware, MiddlewareCollection, NextFn};
use crate::request::Request;
use crate::response::Response;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::ops::{Bound, Deref, RangeBounds};
use std::pin::Pin;
use std::slice::Iter;
use std::sync::Arc;
use std::vec::IntoIter;

/// A segment of a route path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RoutePathSegment {
    /// A static path segment.
    Static(String),
    /// A dynamic, wildcard path segment.
    Wildcard(String),
}

impl RoutePathSegment {
    /// Gets the name of the path segment. For static segments, this returns
    /// "foo" for a path `/foo`. For wildcard segments, this returns "bar" for a
    /// path `/{bar}`.
    pub fn name(&self) -> &str {
        match self {
            Self::Static(name) => name.as_str(),
            Self::Wildcard(name) => name.as_str(),
        }
    }
}

impl Display for RoutePathSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(segment) => f.write_str(segment),
            Self::Wildcard(name) => f.write_str(&format!("{{{}}}", name)),
        }
    }
}

/// A route path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoutePath(Arc<[RoutePathSegment]>);

impl RoutePath {
    /// Creates a new empty route path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Joins two route paths together.
    pub fn join<P>(&self, path: P) -> Self
    where
        P: Into<Self>,
    {
        let other_path: Self = path.into();
        let mut components = self.0.to_vec();
        components.extend(other_path.iter().map(ToOwned::to_owned));
        Self(Arc::from(components))
    }

    /// Returns an iterator over the segments of the route path.
    pub fn iter(&self) -> Iter<'_, RoutePathSegment> {
        self.0.iter()
    }

    /// Returns the number of segments in the path.
    pub fn num_segments(&self) -> usize {
        self.0.len()
    }

    /// Returns a slice of all path segments.
    pub fn segments(&self) -> &[RoutePathSegment] {
        &self.0
    }

    /// Returns a slice of a range path segments.
    pub fn with_segments<R>(&self, range: R) -> &[RoutePathSegment]
    where
        R: RangeBounds<usize>,
    {
        let start = match range.start_bound() {
            Bound::Excluded(bound) => *bound,
            Bound::Included(bound) => bound + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Excluded(bound) => *bound,
            Bound::Included(bound) => bound + 1,
            Bound::Unbounded => self.0.len(),
        };

        &self.0[start..end]
    }

    /// Returns a new route path constructed from the given range of this path's
    /// segments.
    pub fn of_segments<R>(&self, range: R) -> Self
    where
        R: RangeBounds<usize>,
    {
        Self(Arc::from(self.with_segments(range)))
    }

    /// Returns the first segment of the path and a new route path containing
    /// all segments after the first, or `None` if the path is empty.
    pub fn split_first(&self) -> Option<(RoutePathSegment, Self)> {
        self.segments()
            .split_first()
            .map(|(first, rest)| (first.to_owned(), Self::from(rest)))
    }
}

impl Default for RoutePath {
    fn default() -> Self {
        Self(Arc::new([]))
    }
}

impl Display for RoutePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "/{}",
            self.0
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("/")
        ))
    }
}

impl From<&str> for RoutePath {
    fn from(value: &str) -> Self {
        Self(
            value
                .split('/')
                .filter_map(|segment| {
                    if segment.is_empty() {
                        None
                    } else if segment.starts_with('{')
                        && segment.ends_with('}')
                        && segment.len() > 2
                    {
                        Some(RoutePathSegment::Wildcard(
                            segment[1..segment.len() - 1].to_owned(),
                        ))
                    } else {
                        Some(RoutePathSegment::Static(segment.to_owned()))
                    }
                })
                .collect(),
        )
    }
}

impl From<String> for RoutePath {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}

impl From<RoutePathSegment> for RoutePath {
    fn from(value: RoutePathSegment) -> Self {
        Self(Arc::from([value]))
    }
}

impl From<&[RoutePathSegment]> for RoutePath {
    fn from(value: &[RoutePathSegment]) -> Self {
        Self(Arc::from(value))
    }
}

impl Deref for RoutePath {
    type Target = [RoutePathSegment];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<[RoutePathSegment]> for RoutePath {
    fn borrow(&self) -> &[RoutePathSegment] {
        &self.0
    }
}

impl<'a> IntoIterator for &'a RoutePath {
    type Item = &'a RoutePathSegment;
    type IntoIter = Iter<'a, RoutePathSegment>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl FromIterator<RoutePathSegment> for RoutePath {
    fn from_iter<T: IntoIterator<Item = RoutePathSegment>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// A segment of a matched route path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RoutePathMatchedSegment {
    /// A static path segment.
    Static(String),
    /// A dynamic, wildcard path segment.
    Wildcard(String, String),
}

/// A matched route path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoutePathMatched(Arc<[RoutePathMatchedSegment]>);

impl RoutePathMatched {
    /// Creates a new empty matched route path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Joins two route paths together.
    pub fn join<P>(&self, path: P) -> Self
    where
        P: Into<Self>,
    {
        let other_path: Self = path.into();
        let mut components = self.0.to_vec();
        components.extend(other_path.iter().map(ToOwned::to_owned));
        Self(Arc::from(components))
    }

    /// Returns an iterator over the segments of the route path.
    pub fn iter(&self) -> Iter<'_, RoutePathMatchedSegment> {
        self.0.iter()
    }

    /// Returns the number of segments in the path.
    pub fn num_segments(&self) -> usize {
        self.0.len()
    }

    /// Returns a slice of all path segments.
    pub fn segments(&self) -> &[RoutePathMatchedSegment] {
        &self.0
    }

    /// Returns a slice of a range path segments.
    pub fn with_segments<R>(&self, range: R) -> &[RoutePathMatchedSegment]
    where
        R: RangeBounds<usize>,
    {
        let start = match range.start_bound() {
            Bound::Excluded(bound) => *bound,
            Bound::Included(bound) => bound + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Excluded(bound) => *bound,
            Bound::Included(bound) => bound + 1,
            Bound::Unbounded => self.0.len(),
        };

        &self.0[start..end]
    }

    /// Returns a new route path constructed from the given range of this path's
    /// segments.
    pub fn of_segments<R>(&self, range: R) -> Self
    where
        R: RangeBounds<usize>,
    {
        Self(Arc::from(self.with_segments(range)))
    }

    /// Returns the first segment of the path and a new route path containing
    /// all segments after the first, or `None` if the path is empty.
    pub fn split_first(&self) -> Option<(RoutePathMatchedSegment, Self)> {
        self.segments()
            .split_first()
            .map(|(first, rest)| (first.to_owned(), Self::from(rest)))
    }
}

impl Default for RoutePathMatched {
    fn default() -> Self {
        Self(Arc::new([]))
    }
}

impl From<RoutePathMatchedSegment> for RoutePathMatched {
    fn from(value: RoutePathMatchedSegment) -> Self {
        Self(Arc::from([value]))
    }
}

impl From<&[RoutePathMatchedSegment]> for RoutePathMatched {
    fn from(value: &[RoutePathMatchedSegment]) -> Self {
        Self(Arc::from(value))
    }
}

impl Deref for RoutePathMatched {
    type Target = [RoutePathMatchedSegment];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<[RoutePathMatchedSegment]> for RoutePathMatched {
    fn borrow(&self) -> &[RoutePathMatchedSegment] {
        &self.0
    }
}

impl<'a> IntoIterator for &'a RoutePathMatched {
    type Item = &'a RoutePathMatchedSegment;
    type IntoIter = Iter<'a, RoutePathMatchedSegment>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl FromIterator<RoutePathMatchedSegment> for RoutePathMatched {
    fn from_iter<T: IntoIterator<Item = RoutePathMatchedSegment>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// A shareable route handler.
#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct RouteHandler(
    pub(crate) Arc<dyn Fn(Request) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync>,
);

impl RouteHandler {
    /// Creates a new route handler from the provided function.
    fn new<F, Fut>(route: F) -> Self
    where
        F: Fn(Request) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        Self(Arc::new(move |req| Box::pin(route(req))))
    }

    /// Calls the handler.
    pub(crate) async fn call(&self, req: Request) -> Response {
        (self.0)(req).await
    }
}

impl<F, Fut> From<F> for RouteHandler
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    fn from(value: F) -> Self {
        Self::new(value)
    }
}

/// A route handler with middleware included.
#[derive(Clone)]
pub struct CompleteRouteHandler {
    /// The middleware installed on the route.
    middleware: Arc<[Middleware]>,
    /// The inner route handler.
    handler: RouteHandler,
    /// The callable used to invoke the handler and all middleware.
    invoker: NextFn,
}

impl CompleteRouteHandler {
    /// Creates a new route handler with middleware.
    pub fn new(handler: RouteHandler, middleware: Arc<[Middleware]>) -> Self {
        Self {
            middleware: Arc::clone(&middleware),
            handler: handler.clone(),
            invoker: Self::generate_invoker(middleware, handler),
        }
    }

    /// Installs additional middleware onto this route.
    pub fn add_middleware(&mut self, mut middleware: Vec<Middleware>) {
        middleware.extend(self.middleware.to_vec());
        self.middleware = Arc::from(middleware);
        self.invoker = Self::generate_invoker(Arc::clone(&self.middleware), self.handler.clone());
    }

    /// Recursively constructs a callable that can be used to invoke the handler
    /// and all middleware.
    fn generate_invoker_recursive(middleware: &[Middleware], handler: RouteHandler) -> NextFn {
        match middleware.split_first() {
            Some((first, rest)) => NextFn::new({
                let first = first.clone();
                let next = Self::generate_invoker_recursive(rest, handler);
                move |mut req| {
                    let first = first.clone();
                    req.next = Some(next.clone());
                    async move { first.call(req).await }
                }
            }),
            None => NextFn::new(move |mut req| {
                let handler = handler.clone();
                req.next = None;
                async move { handler.call(req).await }
            }),
        }
    }

    /// Generates a callable that can be used to invoke the handler and all
    /// middleware.
    fn generate_invoker(middleware: Arc<[Middleware]>, handler: RouteHandler) -> NextFn {
        Self::generate_invoker_recursive(&middleware, handler)
    }

    /// Calls the handler and all middleware.
    pub(crate) async fn call(&self, req: Request) -> Response {
        self.invoker.call(req).await
    }
}

impl From<RouteHandler> for CompleteRouteHandler {
    fn from(value: RouteHandler) -> Self {
        let middleware = Arc::from([]);
        let handler = value;

        Self {
            middleware: Arc::clone(&middleware),
            handler: handler.clone(),
            invoker: Self::generate_invoker(middleware, handler),
        }
    }
}

/// A recursive structure for containing route handlers.
#[derive(Clone, Default)]
pub struct RouteLevel {
    /// All routes that exist at this level of the routing tree.
    self_routes: HashMap<HttpMethod, CompleteRouteHandler>,
    /// All static subroutes.
    static_sub_routes: HashMap<String, Self>,
    /// An optional named wildcard subroute.
    wildcard_sub_route: Option<(String, Box<Self>)>,
}

impl RouteLevel {
    /// Creates a new empty route level.
    pub fn new() -> Self {
        Self::default()
    }

    /// Recursively retrieves a route from the routing tree and constructs the
    /// matched route path.
    fn get_recursive(
        &self,
        method: HttpMethod,
        path: RoutePath,
        path_match: RoutePathMatched,
    ) -> Result<(RoutePathMatched, CompleteRouteHandler)> {
        match path.split_first() {
            None => self
                .self_routes
                .get(&method)
                .cloned()
                .map(|route| (path_match, route))
                .ok_or(Error::MethodNotAllowed),
            Some((first, rest)) => match first {
                RoutePathSegment::Static(name) => match self.static_sub_routes.get(&name) {
                    Some(routes) => routes.get_recursive(
                        method,
                        rest,
                        path_match.join(RoutePathMatchedSegment::Static(name)),
                    ),
                    None => match &self.wildcard_sub_route {
                        Some((wildcard_name, routes)) => routes.get_recursive(
                            method,
                            rest,
                            path_match.join(RoutePathMatchedSegment::Wildcard(
                                wildcard_name.clone(),
                                name,
                            )),
                        ),
                        None => Err(Error::NotFound),
                    },
                },
                RoutePathSegment::Wildcard(_) => unreachable!(),
            },
        }
    }

    /// Attempts to retrieve a route handler and matched path from the route
    /// tree.
    pub fn get(
        &self,
        method: HttpMethod,
        path: RoutePath,
    ) -> Result<(RoutePathMatched, CompleteRouteHandler)> {
        self.get_recursive(method, path, RoutePathMatched::new())
    }

    /// Adds a route handler to the route tree.
    pub fn add(&mut self, method: HttpMethod, path: RoutePath, handler: CompleteRouteHandler) {
        match path.split_first() {
            None => {
                self.self_routes.insert(method, handler);
            }
            Some((first, rest)) => match first {
                RoutePathSegment::Static(name) => self
                    .static_sub_routes
                    .entry(name)
                    .or_default()
                    .add(method, rest, handler),
                RoutePathSegment::Wildcard(name) => {
                    let mut wildcard_sub_routes = Self::default();
                    wildcard_sub_routes.add(method, rest, handler);
                    self.wildcard_sub_route = Some((name, Box::new(wildcard_sub_routes)));
                }
            },
        }
    }

    /// Merges a group of route handlers into the route tree.
    pub fn add_group(&mut self, group: RouteGroup, middleware: Vec<Middleware>) {
        self.add_level(group.path(), group.into_route_level(), middleware);
    }

    /// Merges a level of route handlers into the route tree.
    pub fn add_level(&mut self, subpath: RoutePath, routes: Self, middleware: Vec<Middleware>) {
        routes
            .flatten()
            .into_iter()
            .for_each(|(method, path, mut handler)| {
                handler.add_middleware(middleware.clone());
                self.add(method, subpath.join(path), handler);
            });
    }

    /// Recursively builds a flat collection of routes from `self`.
    fn flatten_recursive(
        self,
        subpath: RoutePath,
    ) -> Vec<(HttpMethod, RoutePath, CompleteRouteHandler)> {
        let mut routes = Vec::new();

        for (method, route) in self.self_routes {
            routes.push((method, subpath.clone(), route));
        }

        for (subroute_name, subroute) in self.static_sub_routes {
            routes.extend(
                subroute.flatten_recursive(subpath.join(RoutePathSegment::Static(subroute_name))),
            );
        }

        if let Some((subroute_name, subroute)) = self.wildcard_sub_route {
            routes.extend(
                subroute.flatten_recursive(subpath.join(RoutePathSegment::Wildcard(subroute_name))),
            );
        }

        routes
    }

    /// Turns `self` into a flat collection of routes.
    pub fn flatten(self) -> Vec<(HttpMethod, RoutePath, CompleteRouteHandler)> {
        self.flatten_recursive(RoutePath::new())
    }
}

impl IntoIterator for RouteLevel {
    type Item = (HttpMethod, RoutePath, CompleteRouteHandler);
    type IntoIter = IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.flatten().into_iter()
    }
}

impl From<RouteGroup> for RouteLevel {
    fn from(value: RouteGroup) -> Self {
        value.into_route_level()
    }
}

/// A group of routes under a given path.
#[derive(Clone, Default)]
pub struct RouteGroup {
    /// The path of the route group.
    pub(crate) path: RoutePath,
    /// The collection of routes within the group.
    pub(crate) routes: HashMap<(HttpMethod, RoutePath), RouteHandler>,
    /// The collection of route groups within the group.
    pub(crate) groups: Vec<Self>,
    /// The collection of all registered middleware.
    pub(crate) middleware: MiddlewareCollection,
}

impl RouteGroup {
    /// Creates a new empty route group.
    pub fn new<P>(path: P) -> Self
    where
        P: Into<RoutePath>,
    {
        Self {
            path: path.into(),
            routes: HashMap::new(),
            groups: Vec::new(),
            middleware: MiddlewareCollection::new(),
        }
    }

    /// Returns the path of this route group.
    pub fn path(&self) -> RoutePath {
        self.path.clone()
    }

    /// Registers a route within the route group.
    pub fn route<P, R>(mut self, method: HttpMethod, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.routes.insert((method, path.into()), route.into());
        self
    }

    /// Registers a sub-group of routes.
    pub fn route_group(mut self, route_group: Self) -> Self {
        self.groups.push(route_group);
        self
    }

    /// Shorthand for `.route(HttpMethod::Get, ...)`.
    pub fn get<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(HttpMethod::Get, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Head, ...)`.
    pub fn head<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(HttpMethod::Head, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Post, ...)`.
    pub fn post<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(HttpMethod::Post, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Put, ...)`.
    pub fn put<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(HttpMethod::Put, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Delete, ...)`.
    pub fn delete<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(HttpMethod::Delete, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Connect, ...)`.
    pub fn connect<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(HttpMethod::Connect, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Options, ...)`.
    pub fn options<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(HttpMethod::Options, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Trace, ...)`.
    pub fn trace<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(HttpMethod::Trace, path, route)
    }

    /// Shorthand for `.route(HttpMethod::Patch, ...)`.
    pub fn patch<P, R>(self, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<RouteHandler>,
    {
        self.route(HttpMethod::Patch, path, route)
    }

    /// Register middleware to be used on all routes at this level and all route
    /// groups below.
    pub fn with_middleware<M>(mut self, middleware: M) -> Self
    where
        M: Into<Middleware>,
    {
        self.middleware.add_recursive(middleware.into());
        self
    }

    /// Registers middleware to be used on all routes at this level, but not on
    /// route groups below.
    pub fn with_local_middleware<M>(mut self, middleware: M) -> Self
    where
        M: Into<Middleware>,
    {
        self.middleware.add_local(middleware.into());
        self
    }

    /// Turns the route group into a `RouteLevel`, preserving registered
    /// middleware correctly.
    pub fn into_route_level(self) -> RouteLevel {
        let (local_middleware, recursive_middleware) = self.middleware.into_iter().fold(
            (Vec::new(), Vec::new()),
            |(mut local_middleware, mut recursive_middleware), middleware| {
                match middleware {
                    AppliedMiddleware::Local(inner) => local_middleware.push(inner),
                    AppliedMiddleware::Recursive(inner) => {
                        local_middleware.push(inner.clone());
                        recursive_middleware.push(inner);
                    }
                }

                (local_middleware, recursive_middleware)
            },
        );
        let local_middleware = Arc::from(local_middleware);

        let mut route_level = RouteLevel::new();

        for ((method, path), handler) in self.routes {
            route_level.add(
                method,
                path,
                CompleteRouteHandler::new(handler, Arc::clone(&local_middleware)),
            );
        }

        for group in self.groups {
            route_level.add_group(group, recursive_middleware.clone());
        }

        route_level
    }
}
