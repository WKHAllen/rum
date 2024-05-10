//! Crate-level error types.

use crate::http::StatusCode;
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
    /// Returns the default HTTP status code for this error. For client errors
    /// `400 Bad Request` is returned, and for server errors `500 Internal
    /// Server Error` is returned.
    pub fn response_status(&self) -> StatusCode {
        match *self {
            Self::Client => StatusCode::BadRequest,
            Self::Server => StatusCode::InternalServerError,
        }
    }

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
    /// An error occurred while serializing or deserializing JSON data.
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
    /// A `hyper` server error occurred.
    #[error("server error: {0}")]
    ServerError(#[from] hyper::Error),
    /// A query parameter was missing from the request.
    #[error("missing query parameter: '{0}'")]
    MissingQueryParameterError(String),
    /// A header was missing from the request.
    #[error("missing header: '{0}'")]
    MissingHeaderError(String),
    /// An unknown state type was requested from the server state manager.
    #[error("unknown state type: '{0}'")]
    UnknownStateTypeError(&'static str),
}

impl Error {
    /// Did this error occur because of a failure in the client or the server?
    pub fn source(&self) -> ErrorSource {
        match *self {
            Self::JsonError(_)
            | Self::MissingQueryParameterError(_)
            | Self::MissingHeaderError(_) => ErrorSource::Client,
            Self::ServerError(_) | Self::UnknownStateTypeError(_) => ErrorSource::Server,
        }
    }
}

/// The crate-level `Result` type alias.
pub type Result<T> = std::result::Result<T, Error>;
