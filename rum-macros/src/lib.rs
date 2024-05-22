//! Macros for the Rum crate.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

mod transform;

use crate::transform::transform;
use proc_macro::TokenStream;

/// Transforms a function such that it can be used as a Rum route handler.
#[proc_macro_attribute]
pub fn handler(_: TokenStream, item: TokenStream) -> TokenStream {
    transform(item)
}

/// Transforms a function such that it can be used as a Rum middleware function.
#[proc_macro_attribute]
pub fn middleware(_: TokenStream, item: TokenStream) -> TokenStream {
    transform(item)
}
