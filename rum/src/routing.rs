//! Types involving request routing.

use crate::http::HttpMethod;
use crate::request::ServerRequest;
use crate::response::ServerResponse;
use std::borrow::Borrow;
use std::collections::hash_map::{IntoIter as MapIntoIter, Iter as MapIter, IterMut as MapIterMut};
use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::slice::Iter as VecIter;
use std::sync::Arc;

/// A route path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoutePath(Arc<[String]>);

impl RoutePath {
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
    pub fn iter(&self) -> VecIter<'_, String> {
        self.0.iter()
    }
}

impl Display for RoutePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("/{}", self.0.join("/")))
    }
}

impl From<&str> for RoutePath {
    fn from(value: &str) -> Self {
        Self(
            value
                .split('/')
                .filter_map(|component| {
                    if component.is_empty() {
                        None
                    } else {
                        Some(component.to_owned())
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

impl Deref for RoutePath {
    type Target = [String];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<[String]> for RoutePath {
    fn borrow(&self) -> &[String] {
        &self.0
    }
}

impl<'a> IntoIterator for &'a RoutePath {
    type Item = &'a String;
    type IntoIter = VecIter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl FromIterator<String> for RoutePath {
    fn from_iter<T: IntoIterator<Item = String>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// A shareable route handler.
#[allow(clippy::type_complexity)]
#[derive(Clone)]
pub struct Route(
    Arc<
        dyn Fn(ServerRequest) -> Pin<Box<dyn Future<Output = ServerResponse> + Send>> + Send + Sync,
    >,
);

impl Route {
    /// Creates a new route handler from the provided function.
    fn new<F, Fut>(route: F) -> Self
    where
        F: Fn(ServerRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ServerResponse> + Send + 'static,
    {
        Self(Arc::new(move |req| Box::pin(route(req))))
    }

    /// Calls the handler.
    pub(crate) async fn call(&self, req: ServerRequest) -> ServerResponse {
        (self.0)(req).await
    }
}

impl<F, Fut> From<F> for Route
where
    F: Fn(ServerRequest) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ServerResponse> + Send + 'static,
{
    fn from(value: F) -> Self {
        Self::new(value)
    }
}

/// A group of routes under a given path.
#[derive(Clone, Default)]
pub struct RouteGroup(HashMap<(HttpMethod, RoutePath), Route>);

impl RouteGroup {
    /// Creates a new empty route group.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a route within the route group.
    pub fn route<P, R>(mut self, method: HttpMethod, path: P, route: R) -> Self
    where
        P: Into<RoutePath>,
        R: Into<Route>,
    {
        self.0.insert((method, path.into()), route.into());
        self
    }

    /// Registers a sub-group of routes.
    pub fn route_group<P>(mut self, path: P, route_group: Self) -> Self
    where
        P: Into<RoutePath>,
    {
        let path = path.into();
        route_group
            .into_iter()
            .for_each(|((method, inner_path), route)| {
                self.0.insert((method, path.join(inner_path)), route);
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

    /// Returns an iterator over all routes.
    pub fn iter(&self) -> MapIter<'_, (HttpMethod, RoutePath), Route> {
        self.0.iter()
    }

    /// Returns a mutable iterator over all routes.
    pub fn iter_mut(&mut self) -> MapIterMut<'_, (HttpMethod, RoutePath), Route> {
        self.0.iter_mut()
    }
}

impl<'a> IntoIterator for &'a RouteGroup {
    type Item = (&'a (HttpMethod, RoutePath), &'a Route);
    type IntoIter = MapIter<'a, (HttpMethod, RoutePath), Route>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut RouteGroup {
    type Item = (&'a (HttpMethod, RoutePath), &'a mut Route);
    type IntoIter = MapIterMut<'a, (HttpMethod, RoutePath), Route>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl IntoIterator for RouteGroup {
    type Item = ((HttpMethod, RoutePath), Route);
    type IntoIter = MapIntoIter<(HttpMethod, RoutePath), Route>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<((HttpMethod, RoutePath), Route)> for RouteGroup {
    fn from_iter<T: IntoIterator<Item = ((HttpMethod, RoutePath), Route)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}
