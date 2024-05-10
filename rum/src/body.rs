//! Types for extracting request bodies and building response bodies.

use std::borrow::{Borrow, BorrowMut};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// The HTTP request body as a raw byte slice. This `deref`s to `&[u8]`.
#[derive(Debug, Clone)]
pub struct BodyRaw(pub(crate) Arc<[u8]>);

impl Deref for BodyRaw {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<[u8]> for BodyRaw {
    fn borrow(&self) -> &[u8] {
        &self.0
    }
}

/// The HTTP request or response body as a JSON object represented by `T`. `T`
/// must implement `serde`'s `DeserializeOwned` trait for requests, or
/// `Serialize` trait for responses. This `deref`s to `T`, and can be moved out
/// of `self` with [`into_inner`](Self::into_inner).
#[derive(Debug, Clone)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    /// Moves `T` out of this wrapper.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Borrow<T> for Json<T> {
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T> BorrowMut<T> for Json<T> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0
    }
}
