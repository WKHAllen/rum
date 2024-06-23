//! Types involving HTTP responses.

use crate::body::{BodyString, Json};
use crate::cookie::SetCookie;
use crate::error::{Error, ErrorSource, Result};
use crate::http::StatusCode;
use http::header::SET_COOKIE;
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
    pub code: Option<StatusCode>,
    /// The response body.
    pub body: Option<String>,
    /// The response headers.
    pub headers: Option<HashMap<String, Vec<String>>>,
    /// The cookies.
    pub cookies: Option<Vec<SetCookie>>,
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
    /// body.
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
            inner.code = Some(code);
        }

        self
    }

    /// Sets the response body string content.
    pub fn body(mut self, body: &str) -> Self {
        if let Self::Ok(inner) = &mut self {
            inner.body = Some(body.to_owned());
        }

        self
    }

    /// Sets the response body string content if the body is empty.
    pub fn body_or(mut self, body: &str) -> Self {
        if let Self::Ok(inner) = &mut self {
            if inner.body.as_ref().map(String::is_empty).unwrap_or(true) {
                inner.body = Some(body.to_owned());
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
            match serde_json::to_string(&body) {
                Ok(body) => {
                    inner.body = Some(body);
                }
                Err(err) => {
                    self = Self::Err(Arc::new(Error::ServerJsonError(err)));
                }
            }
        }

        self
    }

    /// Sets a response header.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        if let Self::Ok(inner) = &mut self {
            inner
                .headers
                .get_or_insert_with(HashMap::new)
                .entry(name.to_owned())
                .or_default()
                .push(value.to_owned());
        }

        self
    }

    /// Sets a cookie value.
    pub fn cookie(mut self, cookie: SetCookie) -> Self {
        if let Self::Ok(inner) = &mut self {
            inner.cookies.get_or_insert_with(Vec::new).push(cookie);
        }

        self
    }

    /// Combines multiple responses together. When both `self` and `other` have
    /// response components specified, priority goes to `other`.
    pub fn and<T>(mut self, other: T) -> Self
    where
        T: IntoResponse,
    {
        if let Self::Ok(self_inner) = &mut self {
            if let Self::Ok(other_inner) = other.into_response() {
                if let Some(other_code) = other_inner.code {
                    self_inner.code = Some(other_code);
                }

                if let Some(other_body) = other_inner.body {
                    self_inner.body = Some(other_body);
                }

                if let Some(other_headers) = other_inner.headers {
                    other_headers.into_iter().for_each(|(name, values)| {
                        let this_entry = self_inner
                            .headers
                            .get_or_insert_with(HashMap::new)
                            .entry(name.to_owned())
                            .or_default();

                        values
                            .into_iter()
                            .for_each(|value| this_entry.push(value.to_owned()));
                    });
                }

                if let Some(other_cookies) = other_inner.cookies {
                    let cookies = self_inner.cookies.get_or_insert_with(Vec::new);

                    other_cookies
                        .into_iter()
                        .for_each(|cookie| cookies.push(cookie));
                }
            }
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
            Self::Ok(inner) => (
                inner.code.unwrap_or_default(),
                inner.body.unwrap_or_default(),
                inner.headers.unwrap_or_default(),
                inner.cookies.unwrap_or_default(),
            ),
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
        self.as_str().into_response()
    }
}

impl IntoResponse for &String {
    fn into_response(self) -> Response {
        self.as_str().into_response()
    }
}

impl IntoResponse for BodyString {
    fn into_response(self) -> Response {
        self.0.into_response()
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

impl IntoResponse for StatusCode {
    fn into_response(self) -> Response {
        Response::new().status_code(self)
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

/// Implements `IntoResponse` for the given tuple type.
macro_rules! impl_into_response_tuples {
    ( $( $ty:ident ),* $(,)? ) => {
        impl< $( $ty ),* > IntoResponse for ( $( $ty, )* )
        where
            $( $ty: IntoResponse, )*
        {
            fn into_response(self) -> Response {
                #[allow(non_snake_case, unused_parens)]
                let ( $( $ty, )* ) = self;
                Response::new()
                $(
                    .and( $ty )
                )*
            }
        }
    };
}

all_the_tuples!(impl_into_response_tuples);
