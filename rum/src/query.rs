//! Types for extracting request query parameters.

use crate::error::{Error, Result};
use serde::de::DeserializeOwned;
use std::borrow::{Borrow, BorrowMut};
use std::collections::hash_map::Iter;
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::Arc;

/// A representation of a map of query parameters.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct QueryParamMap(pub(crate) Arc<HashMap<String, String>>);

impl QueryParamMap {
    /// Gets a required query parameter value, returning `Err` if the query
    /// parameter is not present. This method is provided for convenience, since
    /// the error can be propagated using `?` from any route handler.
    pub fn get(&self, query: &str) -> Result<&str> {
        self.get_optional(query)
            .ok_or(Error::MissingQueryParameterError(query.to_owned()))
    }

    /// Gets a required query parameter value and attempts to parse it into `T`,
    /// where `T` is any type that implements [`ParseQueryParam`]. If the query
    /// parameter is not present, or if parsing fails, `Err` is returned.
    pub fn get_as<T>(&self, query: &str) -> Result<T>
    where
        T: ParseQueryParam,
    {
        self.get(query).and_then(|value| T::parse(query, value))
    }

    /// Gets an optional query parameter value.
    pub fn get_optional(&self, query: &str) -> Option<&str> {
        self.0.get(query).map(|s| s.as_str())
    }

    /// Gets an optional query parameter value and attempts to parse it into
    /// `T`, where `T` is any type that implements [`ParseQueryParam`]. If
    /// parsing fails, `Err` is returned.
    pub fn get_optional_as<T>(&self, query: &str) -> Result<Option<T>>
    where
        T: ParseQueryParam,
    {
        match self.get_optional(query) {
            Some(value) => Ok(Some(T::parse(query, value)?)),
            None => Ok(None),
        }
    }
}

impl From<&str> for QueryParamMap {
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
                let value = query_iter
                    .next()
                    .and_then(|value| urlencoding::decode(value).ok());

                match (name, value) {
                    (Some(name), Some(value)) => Some((name.to_owned(), value.into_owned())),
                    _ => None,
                }
            })
            .collect()
    }
}

impl From<String> for QueryParamMap {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<HashMap<String, String>> for QueryParamMap {
    fn from(value: HashMap<String, String>) -> Self {
        Self(Arc::new(value))
    }
}

impl<T> From<Option<T>> for QueryParamMap
where
    T: Into<QueryParamMap>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(query) => query.into(),
            None => Self::default(),
        }
    }
}

impl<'a> IntoIterator for &'a QueryParamMap {
    type Item = (&'a String, &'a String);
    type IntoIter = Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromIterator<(String, String)> for QueryParamMap {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self(Arc::new(iter.into_iter().collect()))
    }
}

impl Deref for QueryParamMap {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<HashMap<String, String>> for QueryParamMap {
    fn borrow(&self) -> &HashMap<String, String> {
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

/// Parse a query parameter from its string value. Direct implementation of this
/// trait is discouraged. Please instead implement [`FromStr`], and this trait
/// will be implemented automatically as long as the associated `Err` type
/// implements [`Display`].
pub trait ParseQueryParam: Sized {
    /// Parses the query parameter from its string representation.
    fn parse(name: &str, value: &str) -> Result<Self>;
}

impl<T, E> ParseQueryParam for T
where
    T: FromStr<Err = E>,
    E: Display,
{
    fn parse(name: &str, value: &str) -> Result<Self> {
        match T::from_str(value) {
            Ok(value_parsed) => Ok(value_parsed),
            Err(e) => Err(Error::QueryParameterParseError(
                name.to_owned(),
                e.to_string(),
            )),
        }
    }
}

/// A single required query parameter.
#[cfg(feature = "nightly")]
pub struct QueryParam<const Q: &'static str, T = String>(pub(crate) T)
where
    T: ParseQueryParam;

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> QueryParam<Q, T>
where
    T: ParseQueryParam,
{
    /// Moves the query value out of this wrapper.
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> Deref for QueryParam<Q, T>
where
    T: ParseQueryParam,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> DerefMut for QueryParam<Q, T>
where
    T: ParseQueryParam,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A single optional query parameter.
#[cfg(feature = "nightly")]
pub struct QueryParamOptional<const Q: &'static str, T = String>(pub(crate) Option<T>)
where
    T: ParseQueryParam;

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> QueryParamOptional<Q, T>
where
    T: ParseQueryParam,
{
    /// Moves the query value out of this wrapper.
    pub fn into_inner(self) -> Option<T> {
        self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> Deref for QueryParamOptional<Q, T>
where
    T: ParseQueryParam,
{
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> DerefMut for QueryParamOptional<Q, T>
where
    T: ParseQueryParam,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
