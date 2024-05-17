//! Types involving HTTP requests.

use crate::body::{BodyRaw, Json};
use crate::error::{Error, Result};
#[cfg(feature = "nightly")]
use crate::header::{Header, HeaderOptional, ParseHeader};
use crate::header::{HeaderMap, Headers};
use crate::http::HttpMethod;
#[cfg(feature = "nightly")]
use crate::path::{ParsePathParam, PathParam};
use crate::path::{PathParamMap, PathParams};
#[cfg(feature = "nightly")]
use crate::query::{ParseQueryParam, QueryParam, QueryParamOptional};
use crate::query::{QueryParamMap, QueryParams};
use crate::routing::{RoutePath, RoutePathMatched, RoutePathMatchedSegment};
use crate::state::{State, StateManager};
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::Request as HyperRequest;
use serde::de::DeserializeOwned;
use std::any::type_name;
use std::sync::Arc;

/// An HTTP request. Typically, direct interaction with this type is
/// discouraged. Users are encouraged to use extractors instead.
#[derive(Debug)]
pub struct Request {
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
}

impl Request {
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
        })
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
    pub fn path(&self) -> &RoutePath {
        &self.path
    }

    /// Gets an optional query parameter value.
    pub fn query_optional(&self, query: &str) -> Option<&str> {
        self.query.get_optional(query)
    }

    /// Gets a required query parameter value.
    pub fn query_required(&self, query: &str) -> Result<&str> {
        self.query.get_required(query)
    }

    /// Gets an optional header value.
    pub fn header_optional(&self, header: &str) -> Option<&str> {
        self.headers.get_optional(header)
    }

    /// Gets a required header value.
    pub fn header_required(&self, header: &str) -> Result<&str> {
        self.headers.get_required(header)
    }
}

/// A trait for defining which types can be used as HTTP request extractors.
pub trait FromRequest: Sized {
    /// Performs the extraction from a request.
    fn from_request(req: &Request) -> Result<Self>;
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
            &req.query.0,
        )?)?))
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> FromRequest for QueryParam<Q, T>
where
    T: ParseQueryParam,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(T::parse(Q, req.query_required(Q)?)?))
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> FromRequest for QueryParamOptional<Q, T>
where
    T: ParseQueryParam,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(match req.query_optional(Q) {
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
            &req.headers.0,
        )?)?))
    }
}

#[cfg(feature = "nightly")]
impl<const H: &'static str, T> FromRequest for Header<H, T>
where
    T: ParseHeader,
{
    fn from_request(req: &Request) -> Result<Self> {
        Ok(Self(T::parse(H, req.header_required(H)?)?))
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
