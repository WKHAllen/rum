//! Crate-level error types.

use crate::http::{Method, StatusCode};
use crate::response::{ErrorBody, Response};
use std::collections::HashSet;
use std::str::Utf8Error;
use thiserror::Error;

/// Notes whether it was the client or the server that caused an error. This is
/// often used to determine which HTTP status code to respond with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSource {
    /// The error was caused by the client (e.g. a 400-level error).
    Client,
    /// The error was caused by the server (e.g. a 500-level error).
    Server,
}

impl ErrorSource {
    /// Is this a client error?
    pub fn is_client(&self) -> bool {
        matches!(*self, Self::Client)
    }

    /// Is this a server error?
    pub fn is_server(&self) -> bool {
        matches!(*self, Self::Server)
    }
}

/// The crate-level error type.
#[derive(Error, Debug)]
pub enum Error {
    /// An error occurred while parsing a request body into a string.
    #[error("string parse error: {0}")]
    StringError(#[from] Utf8Error),
    /// An error occurred while serializing or deserializing JSON data on the
    /// client side.
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
    /// A `hyper` server error occurred.
    #[error("server error: {0}")]
    ServerError(#[from] hyper::Error),
    /// A path parameter was missing.
    #[error("missing path parameter: '{0}'")]
    MissingPathParameterError(String),
    /// A query parameter was missing from the request.
    #[error("missing query parameter: '{0}'")]
    MissingQueryParameterError(String),
    /// A header was missing from the request.
    #[error("missing header: '{0}'")]
    MissingHeaderError(String),
    /// A cookie was missing from the request.
    #[error("missing cookie: '{0}'")]
    MissingCookieError(String),
    /// A path parameter failed parsing.
    #[error("failed to parse path parameter '{0}': {1}")]
    PathParameterParseError(String, String),
    /// A query parameter failed parsing.
    #[error("failed to parse query parameter '{0}': {1}")]
    QueryParameterParseError(String, String),
    /// A header failed parsing.
    #[error("failed to parse header '{0}': {1}")]
    HeaderParseError(String, String),
    /// A cookie failed parsing.
    #[error("failed to parse cookie '{0}': {1}")]
    CookieParseError(String, String),
    /// An unknown state type was requested from the server state manager.
    #[error("unknown state type: '{0}'")]
    UnknownStateTypeError(&'static str),
    /// A `NextFn` was attempted to be extracted from the request from within a
    /// route handler function, where there is no next function.
    #[error("there is no next function, as this is a route handler")]
    NoNextFunction,
    /// The requested path could not be found.
    #[error("the requested path could not be found")]
    NotFound,
    /// The requested path exists, but the method requested is not allowed.
    #[error("the requested method is not allowed")]
    MethodNotAllowed(HashSet<Method>),
    /// The request body content does not match the `Content-Type` header, or
    /// the header is not present.
    #[error("the request body content does not match the `Content-Type` header, or the header is not present")]
    UnsupportedMediaType,
    /// An error occurred while serializing or deserializing JSON data on the
    /// server side.
    #[error("server json error: {0}")]
    ServerJsonError(serde_json::Error),
}

impl Error {
    /// Did this error occur because of a failure in the client or the server?
    pub fn source(&self) -> ErrorSource {
        match *self {
            Self::StringError(_)
            | Self::JsonError(_)
            | Self::MissingQueryParameterError(_)
            | Self::MissingHeaderError(_)
            | Self::MissingCookieError(_)
            | Self::PathParameterParseError(_, _)
            | Self::QueryParameterParseError(_, _)
            | Self::HeaderParseError(_, _)
            | Self::CookieParseError(_, _)
            | Self::NotFound
            | Self::MethodNotAllowed(_)
            | Self::UnsupportedMediaType => ErrorSource::Client,
            Self::ServerError(_)
            | Self::MissingPathParameterError(_)
            | Self::UnknownStateTypeError(_)
            | Self::NoNextFunction
            | Self::ServerJsonError(_) => ErrorSource::Server,
        }
    }

    /// Returns the HTTP response status code that should be used when this
    /// error occurs.
    pub fn response_status(&self) -> StatusCode {
        match *self {
            Self::StringError(_)
            | Self::JsonError(_)
            | Self::MissingQueryParameterError(_)
            | Self::MissingHeaderError(_)
            | Self::MissingCookieError(_)
            | Self::PathParameterParseError(_, _)
            | Self::QueryParameterParseError(_, _)
            | Self::HeaderParseError(_, _)
            | Self::CookieParseError(_, _) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::MethodNotAllowed(_) => StatusCode::METHOD_NOT_ALLOWED,
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::ServerError(_)
            | Self::MissingPathParameterError(_)
            | Self::UnknownStateTypeError(_)
            | Self::NoNextFunction
            | Self::ServerJsonError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Creates an HTTP response object from this error.
    pub fn as_response(&self) -> Response {
        let res = Response::new()
            .status_code(self.response_status())
            .body_json(ErrorBody::new(self.to_string()));

        let res = if let Self::MethodNotAllowed(allow) = self {
            res.header(
                "Allow",
                &allow
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
            )
        } else {
            res
        };

        res
    }
}

/// The crate-level `Result` type alias.
pub type Result<T> = std::result::Result<T, Error>;
