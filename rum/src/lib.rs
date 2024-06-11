//! # Rum
//!
//! A high-level web framework that emphasizes simplicity.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]
#![allow(incomplete_features)]
#![cfg_attr(feature = "nightly", feature(adt_const_params))]
#![cfg_attr(feature = "nightly", feature(fn_traits))]
#![cfg_attr(feature = "nightly", feature(unboxed_closures))]

pub mod body;
pub mod cookie;
pub mod error;
pub mod header;
pub mod middleware;
pub mod path;
pub mod query;
pub mod request;
pub mod response;
pub mod routing;
pub mod server;
pub mod state;
mod typemap;

/// General HTTP-related types.
pub mod http {
    pub use http::{Method, StatusCode};
}

/// The crate prelude. This contains the most useful functions and types from
/// the crate.
pub mod prelude {
    pub use crate::body::{BodyRaw, BodyString, Json};
    #[cfg(feature = "nightly")]
    pub use crate::cookie::{Cookie, CookieOptional};
    pub use crate::cookie::{CookieMap, Cookies, SetCookie};
    #[cfg(feature = "nightly")]
    pub use crate::header::{Header, HeaderOptional};
    pub use crate::header::{HeaderMap, Headers};
    pub use crate::http::{Method, StatusCode};
    pub use crate::middleware::{Middleware, NextFn};
    #[cfg(feature = "nightly")]
    pub use crate::path::PathParam;
    pub use crate::path::{PathParamMap, PathParams};
    #[cfg(feature = "nightly")]
    pub use crate::query::{QueryParam, QueryParamOptional};
    pub use crate::query::{QueryParamMap, QueryParams};
    pub use crate::request::{FromRequest, Request};
    pub use crate::response::{IntoResponse, Response};
    pub use crate::routing::RouteGroup;
    pub use crate::server::{
        error_report_stream, shutdown_signal, ErrorReceiver, ErrorSender, Server, ShutdownReceiver,
        ShutdownSender,
    };
    pub use crate::state::{LocalState, State};
    pub use rum_macros::{handler, middleware};
}
