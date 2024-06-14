//! Types involving HTTP requests.

use crate::body::{BodyRaw, BodyString, Json};
#[cfg(feature = "nightly")]
use crate::cookie::{Cookie, CookieOptional};
use crate::cookie::{CookieMap, Cookies, ParseCookie};
use crate::error::{Error, Result};
#[cfg(feature = "nightly")]
use crate::header::{Header, HeaderOptional};
use crate::header::{HeaderMap, Headers, ParseHeader};
use crate::http::Method;
use crate::middleware::NextFn;
#[cfg(feature = "nightly")]
use crate::path::PathParam;
use crate::path::{ParsePathParam, PathParamMap, PathParams};
use crate::query::{ParseQueryParam, QueryParamMap, QueryParams};
#[cfg(feature = "nightly")]
use crate::query::{QueryParam, QueryParamOptional};
use crate::routing::{RoutePath, RoutePathMatched, RoutePathMatchedSegment, RoutePathString};
use crate::state::{LocalState, State, StateManager};
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::header::COOKIE;
use hyper::Request as HyperRequest;
use serde::de::DeserializeOwned;
use std::any::type_name;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

/// The internal request type.
#[derive(Debug)]
pub struct RequestInner {
    /// The raw request body.
    body: Arc<[u8]>,
    /// The request method.
    method: Method,
    /// The request path.
    path: RoutePath,
    /// The matched path parameters.
    matched_path: RoutePathMatched,
    /// The map of path parameters.
    path_params: PathParamMap,
    /// The map of query parameters.
    query: QueryParamMap,
    /// The map of headers.
    headers: HeaderMap,
    /// The map of cookies.
    cookies: CookieMap,
    /// The global application state manager.
    state: StateManager,
    /// The local state manager.
    local_state: LocalState,
}

impl RequestInner {
    /// Attempts to parse a [`hyper::Request`] into `Self`.
    pub(crate) async fn new(
        req: HyperRequest<Incoming>,
        matched_path: RoutePathMatched,
        state: StateManager,
    ) -> Result<Self> {
        let (head, body) = req.into_parts();

        Ok(Self {
            body: Arc::from(body.collect().await?.to_bytes().to_vec()),
            method: Method::from(&head.method),
            path: RoutePath::from(head.uri.path()),
            matched_path: matched_path.clone(),
            path_params: PathParamMap(Arc::new(
                matched_path
                    .iter()
                    .filter_map(|segment| match segment {
                        RoutePathMatchedSegment::Static(_) => None,
                        RoutePathMatchedSegment::Wildcard(name, value) => {
                            Some((name.clone(), value.clone()))
                        }
                    })
                    .collect(),
            )),
            query: QueryParamMap::from(head.uri.query()),
            headers: head
                .headers
                .iter()
                .filter_map(|(name, value)| {
                    Some((name.to_string(), value.to_str().ok()?.to_owned()))
                })
                .collect(),
            cookies: CookieMap::from(head.headers.get_all(COOKIE).into_iter().fold(
                HashMap::new(),
                |mut cookies, cookie| {
                    if let Ok(cookie_str) = cookie.to_str() {
                        cookie_str.split("; ").for_each(|single_cookie| {
                            let mut split_cookie = single_cookie.split('=');

                            if let (Some(name), Some(value)) =
                                (split_cookie.next(), split_cookie.next())
                            {
                                cookies.insert(name.to_owned(), value.to_owned());
                            }
                        });
                    }

                    cookies
                },
            )),
            state,
            local_state: LocalState::new(),
        })
    }

    /// Gets the raw request body.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Gets the request body as a string.
    pub fn body_str(&self) -> Result<&str> {
        Ok(std::str::from_utf8(&self.body)?)
    }

    /// Deserializes the JSON request body into an instance of `T`.
    pub fn body_json<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        Ok(serde_json::from_slice(&self.body)?)
    }

    /// Gets the request method.
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Gets the request path.
    pub fn path(&self) -> RoutePath {
        self.path.clone()
    }

    /// Gets the matched request path.
    pub fn matched_path(&self) -> RoutePathMatched {
        self.matched_path.clone()
    }

    /// Gets a path parameter value.
    pub fn path_param(&self, name: &str) -> Result<&str> {
        self.path_params.get(name)
    }

    /// Gets a path parameter value and attempts to parse it into `T`.
    pub fn path_param_as<T>(&self, name: &str) -> Result<T>
    where
        T: ParsePathParam,
    {
        self.path_params.get_as(name)
    }

    /// Gets a required query parameter value.
    pub fn query_param(&self, query: &str) -> Result<&str> {
        self.query.get(query)
    }

    /// Gets a required query parameter value and attempts to parse it into `T`.
    pub fn query_param_as<T>(&self, query: &str) -> Result<T>
    where
        T: ParseQueryParam,
    {
        self.query.get_as(query)
    }

    /// Gets an optional query parameter value.
    pub fn query_param_optional(&self, query: &str) -> Option<&str> {
        self.query.get_optional(query)
    }

    /// Gets an optional query parameter value and attempts to parse it into
    /// `T`.
    pub fn query_param_optional_as<T>(&self, query: &str) -> Result<Option<T>>
    where
        T: ParseQueryParam,
    {
        self.query.get_optional_as(query)
    }

    /// Gets a required header value.
    pub fn header(&self, header: &str) -> Result<&[String]> {
        self.headers.get(header)
    }

    /// Gets a required header value and attempts to parse it into `T`.
    pub fn header_as<T>(&self, header: &str) -> Result<Vec<T>>
    where
        T: ParseHeader,
    {
        self.headers.get_as(header)
    }

    /// Gets an optional header value.
    pub fn header_optional(&self, header: &str) -> Option<&[String]> {
        self.headers.get_optional(header)
    }

    /// Gets an optional header value and attempts to parse it into `T`.
    pub fn header_optional_as<T>(&self, header: &str) -> Result<Option<Vec<T>>>
    where
        T: ParseHeader,
    {
        self.headers.get_optional_as(header)
    }

    /// Gets a required cookie value.
    pub fn cookie(&self, cookie: &str) -> Result<&str> {
        self.cookies.get(cookie)
    }

    /// Gets a required cookie value and attempts to parse it into `T`.
    pub fn cookie_as<T>(&self, cookie: &str) -> Result<T>
    where
        T: ParseCookie,
    {
        self.cookies.get_as(cookie)
    }

    /// Gets an optional cookie value.
    pub fn cookie_optional(&self, cookie: &str) -> Option<&str> {
        self.cookies.get_optional(cookie)
    }

    /// Gets an optional cookie value and attempts to parse it into `T`.
    pub fn cookie_optional_as<T>(&self, cookie: &str) -> Result<Option<T>>
    where
        T: ParseCookie,
    {
        self.cookies.get_optional_as(cookie)
    }

    /// Gets a value from the global application state.
    pub fn state_value<T>(&self) -> Result<T>
    where
        T: Clone + 'static,
    {
        match self.state.get_cloned::<T>() {
            Some(state) => Ok(state),
            None => Err(Error::UnknownStateTypeError(type_name::<T>())),
        }
    }

    /// Gets the local state manager.
    pub fn local_state(&self) -> LocalState {
        self.local_state.clone()
    }
}

/// An HTTP request. Typically, direct interaction with this type is
/// discouraged. Users are encouraged to use extractors instead.
#[derive(Clone)]
pub struct Request {
    /// The inner request value.
    pub(crate) inner: Arc<RequestInner>,
    /// The next middleware function.
    pub(crate) next: Option<NextFn>,
}

impl Request {
    /// Attempts to parse a [`hyper::Request`] into `Self`.
    pub(crate) async fn new(
        req: HyperRequest<Incoming>,
        matched_path: RoutePathMatched,
        state: StateManager,
    ) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(RequestInner::new(req, matched_path, state).await?),
            next: None,
        })
    }

    /// Gets the next middleware function.
    pub fn next_fn(&self) -> Option<NextFn> {
        self.next.clone()
    }

    /// Extracts a part of the request using [`FromRequest`].
    pub fn extract<T>(&self) -> Result<T>
    where
        T: FromRequest,
    {
        T::from_request(self)
    }
}

impl Deref for Request {
    type Target = RequestInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Borrow<RequestInner> for Request {
    fn borrow(&self) -> &RequestInner {
        &self.inner
    }
}

/// A trait for defining which types can be used as HTTP request extractors.
pub trait FromRequest: Sized {
    /// Performs the extraction from a request.
    fn from_request(req: &Request) -> Result<Self>;
}

impl FromRequest for Request {
    fn from_request(req: &Request) -> Result<Self> {
        Ok(req.clone())
    }
}

impl FromRequest for BodyRaw {
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(Arc::clone(&req.body)))
    }
}

impl FromRequest for BodyString {
    fn from_request(req: &Request) -> Result<Self> {
        match req.header_optional("Content-Type") {
            Some(header) => {
                if header.contains(&"text/plain".to_owned()) {
                    Ok(Self(std::str::from_utf8(&req.body)?.to_owned()))
                } else {
                    Err(Error::UnsupportedMediaType)
                }
            }
            None => Err(Error::UnsupportedMediaType),
        }
    }
}

impl<T> FromRequest for Json<T>
where
    T: DeserializeOwned,
{
    fn from_request(req: &Request) -> Result<Self> {
        match req.header_optional("Content-Type") {
            Some(header) => {
                if header.contains(&"application/json".to_owned()) {
                    Ok(Self(serde_json::from_slice(&req.body)?))
                } else {
                    Err(Error::UnsupportedMediaType)
                }
            }
            None => Err(Error::UnsupportedMediaType),
        }
    }
}

impl FromRequest for Method {
    fn from_request(req: &Request) -> Result<Self> {
        Ok(req.method.clone())
    }
}

impl FromRequest for RoutePathString {
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(req.path.to_string()))
    }
}

impl FromRequest for RoutePath {
    fn from_request(req: &Request) -> Result<Self> {
        Ok(req.path.clone())
    }
}

impl FromRequest for RoutePathMatched {
    fn from_request(req: &Request) -> Result<Self> {
        Ok(req.matched_path.clone())
    }
}

impl FromRequest for PathParamMap {
    fn from_request(req: &Request) -> Result<Self> {
        Ok(req.path_params.clone())
    }
}

impl<T> FromRequest for PathParams<T>
where
    T: DeserializeOwned,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(serde_json::from_value(serde_json::to_value(
            &*req.path_params.0,
        )?)?))
    }
}

#[cfg(feature = "nightly")]
impl<const P: &'static str, T> FromRequest for PathParam<P, T>
where
    T: ParsePathParam,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(T::parse(P, req.path_params.get(P)?)?))
    }
}

impl FromRequest for QueryParamMap {
    fn from_request(req: &Request) -> Result<Self> {
        Ok(req.query.clone())
    }
}

impl<T> FromRequest for QueryParams<T>
where
    T: DeserializeOwned,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(serde_json::from_value(serde_json::to_value(
            &*req.query.0,
        )?)?))
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> FromRequest for QueryParam<Q, T>
where
    T: ParseQueryParam,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(T::parse(Q, req.query_param(Q)?)?))
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> FromRequest for QueryParamOptional<Q, T>
where
    T: ParseQueryParam,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(match req.query_param_optional(Q) {
            Some(value) => Some(T::parse(Q, value)?),
            None => None,
        }))
    }
}

impl FromRequest for HeaderMap {
    fn from_request(req: &Request) -> Result<Self> {
        Ok(req.headers.clone())
    }
}

impl<T> FromRequest for Headers<T>
where
    T: DeserializeOwned,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(serde_json::from_value(serde_json::to_value(
            &*req.headers.0,
        )?)?))
    }
}

#[cfg(feature = "nightly")]
impl<const H: &'static str, T> FromRequest for Header<H, T>
where
    T: ParseHeader,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(req.header_as(H)?))
    }
}

#[cfg(feature = "nightly")]
impl<const H: &'static str, T> FromRequest for HeaderOptional<H, T>
where
    T: ParseHeader,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(req.header_optional_as(H)?))
    }
}

impl FromRequest for CookieMap {
    fn from_request(req: &Request) -> Result<Self> {
        Ok(req.cookies.clone())
    }
}

impl<T> FromRequest for Cookies<T>
where
    T: DeserializeOwned,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(serde_json::from_value(serde_json::to_value(
            &*req.cookies.0,
        )?)?))
    }
}

#[cfg(feature = "nightly")]
impl<const C: &'static str, T> FromRequest for Cookie<C, T>
where
    T: ParseCookie,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(T::parse(C, req.cookie(C)?)?))
    }
}

#[cfg(feature = "nightly")]
impl<const C: &'static str, T> FromRequest for CookieOptional<C, T>
where
    T: ParseCookie,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(match req.cookie_optional(C) {
            Some(value) => Some(T::parse(C, value)?),
            None => None,
        }))
    }
}

impl<T> FromRequest for State<T>
where
    T: Clone + 'static,
{
    fn from_request(req: &Request) -> Result<Self> {
        match req.state.get_cloned::<T>() {
            Some(state) => Ok(Self(state)),
            None => Err(Error::UnknownStateTypeError(type_name::<T>())),
        }
    }
}

impl FromRequest for LocalState {
    fn from_request(req: &Request) -> Result<Self> {
        Ok(req.local_state.clone())
    }
}

impl FromRequest for NextFn {
    fn from_request(req: &Request) -> Result<Self> {
        match &req.next {
            Some(next) => Ok(next.clone()),
            None => Err(Error::NoNextFunction),
        }
    }
}
