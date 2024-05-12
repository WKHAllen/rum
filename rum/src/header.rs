//! Types for extracting request headers.

use crate::error::{Error, Result};
use serde::de::DeserializeOwned;
use std::borrow::{Borrow, BorrowMut};
use std::collections::hash_map::{IntoIter, Iter};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// The inner representation of a map of headers.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HeaderMapInner(pub(crate) HashMap<String, String>);

impl HeaderMapInner {
    /// Gets an optional header value.
    pub fn get_optional(&self, header: &str) -> Option<&str> {
        self.0.get(header).map(|s| s.as_str())
    }

    /// Gets a required header value, returning `Err` if the header is not
    /// present. This method is provided for convenience, since the error can be
    /// propagated using `?` from any route handler.
    pub fn get_required(&self, header: &str) -> Result<&str> {
        self.get_optional(header)
            .ok_or(Error::MissingHeaderError(header.to_owned()))
    }
}

impl From<HashMap<String, String>> for HeaderMapInner {
    fn from(value: HashMap<String, String>) -> Self {
        Self(value)
    }
}

impl<T> From<Option<T>> for HeaderMapInner
where
    T: Into<HeaderMapInner>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(header) => header.into(),
            None => Self::default(),
        }
    }
}

impl<'a> IntoIterator for &'a HeaderMapInner {
    type Item = (&'a String, &'a String);
    type IntoIter = Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl IntoIterator for HeaderMapInner {
    type Item = (String, String);
    type IntoIter = IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(String, String)> for HeaderMapInner {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// A representation of a map of headers.
#[derive(Debug, Clone)]
pub struct HeaderMap(Arc<HeaderMapInner>);

impl<T> From<T> for HeaderMap
where
    T: Into<HeaderMapInner>,
{
    fn from(value: T) -> Self {
        Self(Arc::new(value.into()))
    }
}

impl<I> FromIterator<I> for HeaderMap
where
    HeaderMapInner: FromIterator<I>,
{
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        Self(Arc::new(HeaderMapInner::from_iter(iter)))
    }
}

impl Deref for HeaderMap {
    type Target = HeaderMapInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<HeaderMapInner> for HeaderMap {
    fn borrow(&self) -> &HeaderMapInner {
        &self.0
    }
}

/// A map of headers represented by `T`. `T` must implement `serde`'s
/// `DeserializeOwned` trait, as this is used to deserialize header maps from
/// HTTP requests. This `deref`s to `T`, and can be moved out of `self` with
/// [`into_inner`](Self::into_inner).
///
/// This is limited by implementation details, so all fields in `T` must have
/// values of type `String` or `Option<String>`. Attempts to parse header values
/// from other types will fail.
#[derive(Debug, Clone)]
pub struct Headers<T>(pub(crate) T)
where
    T: DeserializeOwned;

impl<T> Headers<T>
where
    T: DeserializeOwned,
{
    /// Moves `T` out of this wrapper.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Headers<T>
where
    T: DeserializeOwned,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Headers<T>
where
    T: DeserializeOwned,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Borrow<T> for Headers<T>
where
    T: DeserializeOwned,
{
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T> BorrowMut<T> for Headers<T>
where
    T: DeserializeOwned,
{
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// A single required request header.
#[cfg(feature = "nightly")]
pub struct Header<const Q: &'static str>(pub(crate) String);

#[cfg(feature = "nightly")]
impl<const Q: &'static str> Header<Q> {
    /// Moves the header value out of this wrapper.
    pub fn into_inner(self) -> String {
        self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str> Deref for Header<Q> {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str> DerefMut for Header<Q> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A single optional request header.
#[cfg(feature = "nightly")]
pub struct HeaderOptional<const Q: &'static str>(pub(crate) Option<String>);

#[cfg(feature = "nightly")]
impl<const Q: &'static str> HeaderOptional<Q> {
    /// Moves the header value out of this wrapper.
    pub fn into_inner(self) -> Option<String> {
        self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str> Deref for HeaderOptional<Q> {
    type Target = Option<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str> DerefMut for HeaderOptional<Q> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
