//! Types involving HTTP responses.

use crate::body::Json;
use crate::cookie::SetCookie;
use crate::error::{Error, ErrorSource, Result};
use crate::http::StatusCode;
use hyper::header::SET_COOKIE;
use hyper::Response as HyperResponse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
#[derive(Debug, Clone, Default)]
pub struct ResponseInner {
    /// The response status code.
    pub code: StatusCode,
    /// The response body.
    pub body: String,
    /// The response headers.
    pub headers: HashMap<String, Vec<String>>,
    /// The cookies.
    pub cookies: Vec<SetCookie>,
}

/// An HTTP response.
#[derive(Debug, Clone)]
pub enum Response {
    /// A successful response. This can still indicate to the client that an
    /// error occurred, if the status code and body are configured
    /// appropriately.
    Ok(ResponseInner),
    /// An unsuccessful response. A response will still be sent to the client,
    /// but the error will be reported.
    Err(Arc<Error>),
}

impl Response {
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

    /// Sets the response body string content.
    pub fn body(mut self, body: &str) -> Self {
        if let Self::Ok(inner) = &mut self {
            body.clone_into(&mut inner.body);
        }

        self
    }

    /// Sets the response body string content if the body is empty.
    pub fn body_or(mut self, body: &str) -> Self {
        if let Self::Ok(inner) = &mut self {
            if inner.body.is_empty() {
                body.clone_into(&mut inner.body);
            }
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
            inner
                .headers
                .entry(name.to_owned())
                .or_default()
                .push(value.to_owned());
        }

        self
    }

    /// Sets a cookie value.
    pub fn cookie(mut self, cookie: SetCookie) -> Self {
        if let Self::Ok(inner) = &mut self {
            inner.cookies.push(cookie);
        }

        self
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::Ok(ResponseInner::default())
    }
}

#[allow(clippy::from_over_into)]
impl Into<HyperResponse<String>> for Response {
    fn into(self) -> HyperResponse<String> {
        let (code, body, headers, cookies) = match self {
            Self::Ok(inner) => (inner.code, inner.body, inner.headers, inner.cookies),
            Self::Err(err) => (
                err.response_status(),
                ErrorBody::new(match err.source() {
                    ErrorSource::Client => err.to_string(),
                    ErrorSource::Server => "An internal error occurred".to_owned(),
                })
                .to_json()
                .unwrap(),
                HashMap::new(),
                Vec::new(),
            ),
        };

        let res = HyperResponse::builder().status(code);

        let res = headers.into_iter().fold(res, |res, (name, values)| {
            values
                .into_iter()
                .fold(res, |res, value| res.header(name.clone(), value))
        });

        let res = cookies.into_iter().fold(res, |res, cookie| {
            res.header(SET_COOKIE, cookie.to_cookie_string())
        });

        res.body(body).unwrap()
    }
}

/// A trait for defining which types can be used as HTTP responses.
pub trait IntoResponse {
    /// Performs the conversion to a response.
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for &str {
    fn into_response(self) -> Response {
        Response::new()
            .body(self)
            .header("Content-Type", "text/plain")
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Response::new()
            .body(self.as_str())
            .header("Content-Type", "text/plain")
    }
}

impl IntoResponse for &String {
    fn into_response(self) -> Response {
        Response::new()
            .body(self)
            .header("Content-Type", "text/plain")
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        Response::new()
            .body_json(self.0)
            .header("Content-Type", "application/json")
    }
}

impl<T> IntoResponse for (StatusCode, T)
where
    T: IntoResponse,
{
    fn into_response(self) -> Response {
        self.1.into_response().status_code(self.0)
    }
}

impl<T> IntoResponse for Result<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> Response {
        match self {
            Ok(value) => value.into_response(),
            Err(err) => Response::new_error(err),
        }
    }
}
