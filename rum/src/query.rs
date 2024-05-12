//! Types for extracting request query parameters.

use crate::error::{Error, Result};
use serde::de::DeserializeOwned;
use std::borrow::{Borrow, BorrowMut};
use std::collections::hash_map::{IntoIter, Iter};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// The inner representation of a map of query parameters.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueryParamMapInner(pub(crate) HashMap<String, String>);

impl QueryParamMapInner {
    /// Gets an optional query parameter value.
    pub fn get_optional(&self, query: &str) -> Option<&str> {
        self.0.get(query).map(|s| s.as_str())
    }

    /// Gets a required query parameter value, returning `Err` if the query
    /// parameter is not present. This method is provided for convenience, since
    /// the error can be propagated using `?` from any route handler.
    pub fn get_required(&self, query: &str) -> Result<&str> {
        self.get_optional(query)
            .ok_or(Error::MissingQueryParameterError(query.to_owned()))
    }
}

impl From<&str> for QueryParamMapInner {
    fn from(value: &str) -> Self {
        let queries = if value.contains('?') {
            value.split('?').nth(1).unwrap()
        } else {
            value
        };

        queries
            .split('&')
            .filter_map(|query| {
                let mut query_iter = query.split('=');
                let name = query_iter.next();
                let value = query_iter.next();

                match (name, value) {
                    (Some(name), Some(value)) => Some((name.to_owned(), value.to_owned())),
                    _ => None,
                }
            })
            .collect()
    }
}

impl From<String> for QueryParamMapInner {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<HashMap<String, String>> for QueryParamMapInner {
    fn from(value: HashMap<String, String>) -> Self {
        Self(value)
    }
}

impl<T> From<Option<T>> for QueryParamMapInner
where
    T: Into<QueryParamMapInner>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(query) => query.into(),
            None => Self::default(),
        }
    }
}

impl<'a> IntoIterator for &'a QueryParamMapInner {
    type Item = (&'a String, &'a String);
    type IntoIter = Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl IntoIterator for QueryParamMapInner {
    type Item = (String, String);
    type IntoIter = IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(String, String)> for QueryParamMapInner {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// A representation of a map of query parameters.
#[derive(Debug, Clone)]
pub struct QueryParamMap(Arc<QueryParamMapInner>);

impl<T> From<T> for QueryParamMap
where
    T: Into<QueryParamMapInner>,
{
    fn from(value: T) -> Self {
        Self(Arc::new(value.into()))
    }
}

impl<I> FromIterator<I> for QueryParamMap
where
    QueryParamMapInner: FromIterator<I>,
{
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        Self(Arc::new(QueryParamMapInner::from_iter(iter)))
    }
}

impl Deref for QueryParamMap {
    type Target = QueryParamMapInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<QueryParamMapInner> for QueryParamMap {
    fn borrow(&self) -> &QueryParamMapInner {
        &self.0
    }
}

/// A map of query parameters represented by `T`. `T` must implement `serde`'s
/// `DeserializeOwned` trait, as this is used to deserialize query parameter
/// maps from HTTP requests. This `deref`s to `T`, and can be moved out of
/// `self` with [`into_inner`](Self::into_inner).
///
/// This is limited by implementation details, so all fields in `T` must have
/// values of type `String` or `Option<String>`. Attempts to parse query
/// parameter values from other types will fail.
#[derive(Debug, Clone)]
pub struct QueryParams<T>(pub(crate) T)
where
    T: DeserializeOwned;

impl<T> QueryParams<T>
where
    T: DeserializeOwned,
{
    /// Moves `T` out of this wrapper.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for QueryParams<T>
where
    T: DeserializeOwned,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for QueryParams<T>
where
    T: DeserializeOwned,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Borrow<T> for QueryParams<T>
where
    T: DeserializeOwned,
{
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T> BorrowMut<T> for QueryParams<T>
where
    T: DeserializeOwned,
{
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// A single required query parameter.
#[cfg(feature = "nightly")]
pub struct Query<const Q: &'static str>(pub(crate) String);

#[cfg(feature = "nightly")]
impl<const Q: &'static str> Query<Q> {
    /// Moves the query value out of this wrapper.
    pub fn into_inner(self) -> String {
        self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str> Deref for Query<Q> {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str> DerefMut for Query<Q> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A single optional query parameter.
#[cfg(feature = "nightly")]
pub struct QueryOptional<const Q: &'static str>(pub(crate) Option<String>);

#[cfg(feature = "nightly")]
impl<const Q: &'static str> QueryOptional<Q> {
    /// Moves the query value out of this wrapper.
    pub fn into_inner(self) -> Option<String> {
        self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str> Deref for QueryOptional<Q> {
    type Target = Option<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str> DerefMut for QueryOptional<Q> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
