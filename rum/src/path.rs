//! Types for extracting route path parameters.

use crate::error::{Error, Result};
use serde::de::DeserializeOwned;
use std::borrow::{Borrow, BorrowMut};
use std::collections::hash_map::Iter;
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::Arc;

/// A representation of a map of path parameters.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PathParamMap(pub(crate) Arc<HashMap<String, String>>);

impl PathParamMap {
    /// Gets a path parameter value, returning `Err` if the path parameter is
    /// not present. This method is provided for convenience, since the error
    /// can be propagated using `?` from any route handler.
    pub fn get(&self, name: &str) -> Result<&str> {
        self.0
            .get(name)
            .map(|s| s.as_str())
            .ok_or(Error::MissingPathParameterError(name.to_owned()))
    }

    /// Returns an iterator over all path parameters.
    pub fn iter(&self) -> Iter<'_, String, String> {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a PathParamMap {
    type Item = (&'a String, &'a String);
    type IntoIter = Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// A map of path parameters represented by `T`. `T` must implement `serde`'s
/// `DeserializeOwned` trait, as this is used to deserialize path parameters
/// into `T`. This `deref`s to `T`, and can be moved out of `self` with
/// [`into_inner`](Self::into_inner).
///
/// This is limited by implementation details, so all fields in `T` must have
/// values of type `String` or `Option<String>`. Attempts to parse path
/// parameter values from other types will fail.
pub struct PathParams<T>(pub(crate) T)
where
    T: DeserializeOwned;

impl<T> PathParams<T>
where
    T: DeserializeOwned,
{
    /// Moves `T` out of this wrapper.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for PathParams<T>
where
    T: DeserializeOwned,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for PathParams<T>
where
    T: DeserializeOwned,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Borrow<T> for PathParams<T>
where
    T: DeserializeOwned,
{
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T> BorrowMut<T> for PathParams<T>
where
    T: DeserializeOwned,
{
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// Parse a path parameter from its string value. Direct implementation of this
/// trait is discouraged. Please instead implement [`FromStr`], and this trait
/// will be implemented automatically as long as the associated `Err` type
/// implements [`Display`].
pub trait ParsePathParam: Sized {
    /// Parses the path parameter from its string representation.
    fn parse(name: &str, value: &str) -> Result<Self>;
}

impl<T, E> ParsePathParam for T
where
    T: FromStr<Err = E>,
    E: Display,
{
    fn parse(name: &str, value: &str) -> Result<Self> {
        match T::from_str(value) {
            Ok(value_parsed) => Ok(value_parsed),
            Err(e) => Err(Error::PathParameterParseError(
                name.to_owned(),
                e.to_string(),
            )),
        }
    }
}

/// A single path parameter.
#[cfg(feature = "nightly")]
pub struct PathParam<const P: &'static str, T = String>(pub(crate) T)
where
    T: ParsePathParam;

#[cfg(feature = "nightly")]
impl<const P: &'static str, T> PathParam<P, T>
where
    T: ParsePathParam,
{
    /// Moves the path parameter value out of this wrapper.
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[cfg(feature = "nightly")]
impl<const P: &'static str, T> Deref for PathParam<P, T>
where
    T: ParsePathParam,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "nightly")]
impl<const P: &'static str, T> DerefMut for PathParam<P, T>
where
    T: ParsePathParam,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
