//! Types involving HTTP requests.

use crate::body::{BodyRaw, Json};
use crate::error::{Error, Result};
#[cfg(feature = "nightly")]
use crate::header::{Header, HeaderOptional, ParseHeader};
use crate::header::{HeaderMap, Headers};
use crate::http::HttpMethod;
use crate::middleware::NextFn;
#[cfg(feature = "nightly")]
use crate::path::{ParsePathParam, PathParam};
use crate::path::{PathParamMap, PathParams};
#[cfg(feature = "nightly")]
use crate::query::{ParseQueryParam, QueryParam, QueryParamOptional};
use crate::query::{QueryParamMap, QueryParams};
use crate::routing::{RoutePath, RoutePathMatched, RoutePathMatchedSegment};
use crate::state::{LocalState, State, StateManager};
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::Request as HyperRequest;
use serde::de::DeserializeOwned;
use std::any::type_name;
use std::borrow::Borrow;
use std::ops::Deref;
use std::sync::Arc;

/// The internal request type.
#[derive(Debug)]
pub struct RequestInner {
    /// The raw request body.
    body: Arc<[u8]>,
    /// The request method.
    method: HttpMethod,
    /// The request path.
    path: RoutePath,
    /// The map of path parameters.
    path_params: PathParamMap,
    /// The map of query parameters.
    query: QueryParamMap,
    /// The map of headers.
    headers: HeaderMap,
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
            method: HttpMethod::from(&head.method),
            path: RoutePath::from(head.uri.path()),
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
                .into_iter()
                .filter_map(|(maybe_name, maybe_value)| {
                    maybe_name.and_then(|name| {
                        maybe_value
                            .to_str()
                            .ok()
                            .map(|value| (name.to_string(), value.to_owned()))
                    })
                })
                .collect(),
            state,
            local_state: LocalState::new(),
        })
    }

    /// Gets the raw request body.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Deserializes the JSON request body into an instance of `T`.
    pub fn body_json<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        Ok(serde_json::from_slice(&self.body)?)
    }

    /// Gets the request method.
    pub fn method(&self) -> HttpMethod {
        self.method
    }

    /// Gets the request path.
    pub fn path(&self) -> RoutePath {
        self.path.clone()
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
    pub fn header(&self, header: &str) -> Result<&str> {
        self.headers.get(header)
    }

    /// Gets a required header value and attempts to parse it into `T`.
    pub fn header_as<T>(&self, header: &str) -> Result<T>
    where
        T: ParseHeader,
    {
        self.headers.get_as(header)
    }

    /// Gets an optional header value.
    pub fn header_optional(&self, header: &str) -> Option<&str> {
        self.headers.get_optional(header)
    }

    /// Gets an optional header value and attempts to parse it into `T`.
    pub fn header_optional_as<T>(&self, header: &str) -> Result<Option<T>>
    where
        T: ParseHeader,
    {
        self.headers.get_optional_as(header)
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

impl<T> FromRequest for Json<T>
where
    T: DeserializeOwned,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(serde_json::from_slice(&req.body)?))
    }
}

impl FromRequest for HttpMethod {
    fn from_request(req: &Request) -> Result<Self> {
        Ok(req.method)
    }
}

impl FromRequest for RoutePath {
    fn from_request(req: &Request) -> Result<Self> {
        Ok(req.path.clone())
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
        Ok(Self(T::parse(H, req.header(H)?)?))
    }
}

#[cfg(feature = "nightly")]
impl<const H: &'static str, T> FromRequest for HeaderOptional<H, T>
where
    T: ParseHeader,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(match req.header_optional(H) {
            Some(value) => Some(T::parse(H, value)?),
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
