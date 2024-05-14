//! Types involving HTTP responses.

use crate::body::Json;
use crate::error::{Error, ErrorSource, Result};
use crate::header::HeaderMapInner;
use crate::http::StatusCode;
use hyper::Response;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::sync::Arc;

/// An internal type used to return error responses to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ErrorBody {
    /// The error message.
    error: String,
}

impl ErrorBody {
    /// Creates a new error response body.
    pub fn new<S>(err: S) -> Self
    where
        S: Display,
    {
        Self {
            error: err.to_string(),
        }
    }

    /// Serializes the error to a JSON string.
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

/// The internal representation of an HTTP response.
#[derive(Debug, Clone)]
pub struct ServerResponseInner {
    /// The response status code.
    pub(crate) code: StatusCode,
    /// The response body.
    pub(crate) body: String,
    /// The response headers.
    pub(crate) headers: HeaderMapInner,
}

impl Default for ServerResponseInner {
    fn default() -> Self {
        Self {
            code: StatusCode::Ok,
            body: "{}".to_owned(),
            headers: HeaderMapInner::default(),
        }
    }
}

/// An HTTP response.
#[derive(Debug, Clone)]
pub enum ServerResponse {
    /// A successful response. This can still indicate to the client that an
    /// error occurred, if the status code and body are configured
    /// appropriately.
    Ok(ServerResponseInner),
    /// An unsuccessful response. A response will still be sent to the client,
    /// but the error will be reported.
    Err(Arc<Error>),
}

impl ServerResponse {
    /// Creates a new success response value, with status code 200 and an empty
    /// JSON object body.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new error response value from the given error.
    pub fn new_error(err: Error) -> Self {
        Self::Err(Arc::new(err))
    }

    /// Sets the response status code.
    pub fn status_code(mut self, code: StatusCode) -> Self {
        if let Self::Ok(inner) = &mut self {
            inner.code = code;
        }

        self
    }

    /// Sets the response body JSON content.
    pub fn body_json<T>(mut self, body: T) -> Self
    where
        T: Serialize,
    {
        if let Self::Ok(inner) = &mut self {
            if let Ok(body) = serde_json::to_string(&body) {
                inner.body = body;
            }
        }

        self
    }

    /// Sets a response header.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        if let Self::Ok(inner) = &mut self {
            inner.headers.0.insert(name.to_owned(), value.to_owned());
        }

        self
    }
}

impl Default for ServerResponse {
    fn default() -> Self {
        Self::Ok(ServerResponseInner::default())
    }
}

#[allow(clippy::from_over_into)]
impl Into<Response<String>> for ServerResponse {
    fn into(self) -> Response<String> {
        let (code, body, headers) = match self {
            Self::Ok(inner) => (inner.code, inner.body, inner.headers),
            Self::Err(err) => (
                err.source().response_status(),
                ErrorBody::new(match err.source() {
                    ErrorSource::Client => err.to_string(),
                    ErrorSource::Server => "An internal error occurred".to_owned(),
                })
                .to_json()
                .unwrap(),
                HeaderMapInner::default(),
            ),
        };

        let res = Response::builder()
            .status(code.code())
            .header("Content-Type", "application/json");

        let res = headers
            .into_iter()
            .fold(res, |res, (name, value)| res.header(name, value));

        res.body(body).unwrap()
    }
}

/// A trait for defining which types can be used as HTTP responses.
pub trait IntoResponse {
    /// Performs the conversion to a response.
    fn into_response(self) -> ServerResponse;
}

impl IntoResponse for ServerResponse {
    fn into_response(self) -> ServerResponse {
        self
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> ServerResponse {
        ServerResponse::new().body_json(self.0)
    }
}

impl<T> IntoResponse for (StatusCode, T)
where
    T: IntoResponse,
{
    fn into_response(self) -> ServerResponse {
        self.1.into_response().status_code(self.0)
    }
}

impl<T> IntoResponse for Result<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> ServerResponse {
        match self {
            Ok(value) => value.into_response(),
            Err(err) => ServerResponse::new_error(err),
        }
    }
}
