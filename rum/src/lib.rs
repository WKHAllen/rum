//! # Rum
//!
//! A high-level web framework that emphasizes simplicity.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

pub mod body;
pub mod error;
pub mod headers;
pub mod http;
pub mod query;
pub mod request;
pub mod response;
pub mod routing;
pub mod server;
pub mod state;
mod typemap;

/// The crate prelude. This contains the most useful functions and types from
/// the crate.
pub mod prelude {
    pub use crate::body::{BodyRaw, Json};
    pub use crate::headers::{HeaderMap, Headers};
    pub use crate::http::{HttpMethod, StatusCode};
    pub use crate::query::{QueryParamMap, QueryParams};
    pub use crate::request::FromRequest;
    pub use crate::response::{IntoResponse, ServerResponse};
    pub use crate::routing::RouteGroup;
    pub use crate::server::{
        error_report_stream, shutdown_signal, ErrorReceiver, ErrorSender, Server, ShutdownReceiver,
        ShutdownSender,
    };
    pub use crate::state::State;
    pub use rum_macros::handler;
}
