//! Types involving HTTP requests.

use crate::body::{BodyRaw, Json};
use crate::error::{Error, Result};
use crate::headers::{HeaderMap, Headers};
use crate::http::HttpMethod;
#[cfg(feature = "nightly")]
use crate::query::{Query, QueryOptional};
use crate::query::{QueryParamMap, QueryParams};
use crate::routing::RoutePath;
use crate::state::{State, StateManager};
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::Request;
use serde::de::DeserializeOwned;
use std::any::type_name;
use std::sync::Arc;

/// An HTTP request. Typically, direct interaction with this type is
/// discouraged. Users are encouraged to use extractors instead.
#[derive(Debug)]
pub struct ServerRequest {
    /// The raw request body.
    body: Arc<[u8]>,
    /// The request method.
    method: HttpMethod,
    /// The request path.
    path: RoutePath,
    /// The map of query parameters.
    query: QueryParamMap,
    /// The map of headers.
    headers: HeaderMap,
    /// The global application state manager.
    state: StateManager,
}

impl ServerRequest {
    /// Attempts to parse a [`hyper::Request`] into `Self`.
    pub(crate) async fn new(req: Request<Incoming>, state: StateManager) -> Result<Self> {
        let (head, body) = req.into_parts();

        Ok(Self {
            body: Arc::from(body.collect().await?.to_bytes().to_vec()),
            method: HttpMethod::from(&head.method),
            path: RoutePath::from(head.uri.path()),
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
    fn from_request(req: &ServerRequest) -> Result<Self>;
}

impl FromRequest for BodyRaw {
    fn from_request(req: &ServerRequest) -> Result<Self> {
        Ok(Self(Arc::clone(&req.body)))
    }
}

impl<T> FromRequest for Json<T>
where
    T: DeserializeOwned,
{
    fn from_request(req: &ServerRequest) -> Result<Self> {
        Ok(Self(serde_json::from_slice(&req.body)?))
    }
}

impl FromRequest for HttpMethod {
    fn from_request(req: &ServerRequest) -> Result<Self> {
        Ok(req.method)
    }
}

impl FromRequest for RoutePath {
    fn from_request(req: &ServerRequest) -> Result<Self> {
        Ok(req.path.clone())
    }
}

impl FromRequest for QueryParamMap {
    fn from_request(req: &ServerRequest) -> Result<Self> {
        Ok(req.query.clone())
    }
}

impl<T> FromRequest for QueryParams<T>
where
    T: DeserializeOwned,
{
    fn from_request(req: &ServerRequest) -> Result<Self> {
        Ok(Self(serde_json::from_value(serde_json::to_value(
            &req.query.0,
        )?)?))
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str> FromRequest for Query<Q> {
    fn from_request(req: &ServerRequest) -> Result<Self> {
        Ok(Query(req.query_required(Q)?.to_owned()))
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str> FromRequest for QueryOptional<Q> {
    fn from_request(req: &ServerRequest) -> Result<Self> {
        Ok(QueryOptional(req.query_optional(Q).map(ToOwned::to_owned)))
    }
}

impl FromRequest for HeaderMap {
    fn from_request(req: &ServerRequest) -> Result<Self> {
        Ok(req.headers.clone())
    }
}

impl<T> FromRequest for Headers<T>
where
    T: DeserializeOwned,
{
    fn from_request(req: &ServerRequest) -> Result<Self> {
        Ok(Self(serde_json::from_value(serde_json::to_value(
            &req.headers.0,
        )?)?))
    }
}

impl<T> FromRequest for State<T>
where
    T: Clone + 'static,
{
    fn from_request(req: &ServerRequest) -> Result<Self> {
        match req.state.get_cloned::<T>() {
            Some(state) => Ok(State(state)),
            None => Err(Error::UnknownStateTypeError(type_name::<T>())),
        }
    }
}
