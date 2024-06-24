//! Types and extractors for cookies.

use crate::error::{Error, Result};
use serde::de::DeserializeOwned;
use std::borrow::{Borrow, BorrowMut};
use std::collections::hash_map::Iter;
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A representation of a map of cookies.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CookieMap(pub(crate) Arc<HashMap<String, String>>);

impl CookieMap {
    /// Gets a required cookie value, returning `Err` if the cookie is not
    /// present. This method is provided for convenience, since the error can be
    /// propagated using `?` from any route handler.
    pub fn get(&self, cookie: &str) -> Result<&str> {
        self.get_optional(cookie)
            .ok_or(Error::MissingCookieError(cookie.to_owned()))
    }

    /// Gets a required cookie value and attempts to parse it into `T`, where
    /// `T` is any type that implements [`ParseCookie`]. If the cookie parameter
    /// is not present, or if parsing fails, `Err` is returned.
    pub fn get_as<T>(&self, cookie: &str) -> Result<T>
    where
        T: ParseCookie,
    {
        self.get(cookie).and_then(|value| T::parse(cookie, value))
    }

    /// Gets an optional cookie value.
    pub fn get_optional(&self, cookie: &str) -> Option<&str> {
        self.0.get(cookie).map(|s| s.as_str())
    }

    /// Gets an optional cookie value and attempts to parse it into `T`, where
    /// `T` is any type that implements [`ParseCookie`]. If parsing fails, `Err`
    /// is returned.
    pub fn get_optional_as<T>(&self, cookie: &str) -> Result<Option<T>>
    where
        T: ParseCookie,
    {
        match self.get_optional(cookie) {
            Some(value) => Ok(Some(T::parse(cookie, value)?)),
            None => Ok(None),
        }
    }
}

impl From<HashMap<String, String>> for CookieMap {
    fn from(value: HashMap<String, String>) -> Self {
        Self(Arc::new(value))
    }
}

impl<T> From<Option<T>> for CookieMap
where
    T: Into<CookieMap>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(cookie) => cookie.into(),
            None => Self::default(),
        }
    }
}

impl<'a> IntoIterator for &'a CookieMap {
    type Item = (&'a String, &'a String);
    type IntoIter = Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl FromIterator<(String, String)> for CookieMap {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self(Arc::new(iter.into_iter().collect()))
    }
}

impl Deref for CookieMap {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<HashMap<String, String>> for CookieMap {
    fn borrow(&self) -> &HashMap<String, String> {
        &self.0
    }
}

/// A map of cookies represented by `T`. `T` must implement `serde`'s
/// `DeserializeOwned` trait, as this is used to deserialize cookie maps from
/// HTTP requests. This `deref`s to `T`, and can be moved out of `self` with
/// [`into_inner`](Self::into_inner).
///
/// This is limited by implementation details, so all fields in `T` must have
/// values of type `String` or `Option<String>`. Attempts to parse cookie values
/// from other types will fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cookies<T>(pub(crate) T)
where
    T: DeserializeOwned;

impl<T> Cookies<T>
where
    T: DeserializeOwned,
{
    /// Moves `T` out of this wrapper.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Cookies<T>
where
    T: DeserializeOwned,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Cookies<T>
where
    T: DeserializeOwned,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Borrow<T> for Cookies<T>
where
    T: DeserializeOwned,
{
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T> BorrowMut<T> for Cookies<T>
where
    T: DeserializeOwned,
{
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// Parse a cookie from its string value. Direct implementation of this trait is
/// discouraged. Please instead implement [`FromStr`], and this trait will be
/// implemented automatically as long as the associated `Err` type implements
/// [`Display`].
pub trait ParseCookie: Sized {
    /// Parses the cookie from its string representation.
    fn parse(name: &str, value: &str) -> Result<Self>;
}

impl<T, E> ParseCookie for T
where
    T: FromStr<Err = E>,
    E: Display,
{
    fn parse(name: &str, value: &str) -> Result<Self> {
        T::from_str(value).map_err(|err| Error::CookieParseError(name.to_owned(), err.to_string()))
    }
}

/// A single required cookie.
#[cfg(feature = "nightly")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cookie<const C: &'static str, T = String>(pub(crate) T)
where
    T: ParseCookie;

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> Cookie<Q, T>
where
    T: ParseCookie,
{
    /// Moves the cookie value out of this wrapper.
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> Deref for Cookie<Q, T>
where
    T: ParseCookie,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> DerefMut for Cookie<Q, T>
where
    T: ParseCookie,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> Borrow<T> for Cookie<Q, T>
where
    T: ParseCookie,
{
    fn borrow(&self) -> &T {
        &self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> BorrowMut<T> for Cookie<Q, T>
where
    T: ParseCookie,
{
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// A single optional cookie.
#[cfg(feature = "nightly")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CookieOptional<const C: &'static str, T = String>(pub(crate) Option<T>)
where
    T: ParseCookie;

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> CookieOptional<Q, T>
where
    T: ParseCookie,
{
    /// Moves the cookie value out of this wrapper.
    pub fn into_inner(self) -> Option<T> {
        self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> Deref for CookieOptional<Q, T>
where
    T: ParseCookie,
{
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> DerefMut for CookieOptional<Q, T>
where
    T: ParseCookie,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> Borrow<Option<T>> for CookieOptional<Q, T>
where
    T: ParseCookie,
{
    fn borrow(&self) -> &Option<T> {
        &self.0
    }
}

#[cfg(feature = "nightly")]
impl<const Q: &'static str, T> BorrowMut<Option<T>> for CookieOptional<Q, T>
where
    T: ParseCookie,
{
    fn borrow_mut(&mut self) -> &mut Option<T> {
        &mut self.0
    }
}

/// Configuration for setting a cookie.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SetCookie {
    /// The name of the cookie.
    pub(crate) name: String,
    /// The cookie value.
    pub(crate) value: String,
    /// The duration of time for the cookie to exist. If not specified, the
    /// cookie will expire at the end of the session.
    pub(crate) max_age: Option<Duration>,
    /// Whether the cookie is http-only.
    pub(crate) http_only: bool,
}

impl SetCookie {
    /// Creates a new cookie with the given name and value.
    pub fn new<T>(name: &str, value: T) -> Self
    where
        T: Display,
    {
        Self {
            name: name.to_owned(),
            value: value.to_string(),
            max_age: None,
            http_only: false,
        }
    }

    /// Sets the timestamp at which the cookie will expire.
    pub fn expire_at(mut self, timestamp: Instant) -> Self {
        self.max_age = Some(timestamp - Instant::now());
        self
    }

    /// Sets the duration of time the cookie will exist before expiring.
    pub fn expire_after(mut self, duration: Duration) -> Self {
        self.max_age = Some(duration);
        self
    }

    /// Sets whether the cookie should be http-only.
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }

    /// Returns a string that can be used to set the cookie in an HTTP response.
    pub fn to_cookie_string(&self) -> String {
        let mut cookie_str = format!("{}={}", self.name, self.value);

        if let Some(max_age) = self.max_age {
            cookie_str.push_str(&format!("; Max-Age={}", max_age.as_secs()));
        }

        if self.http_only {
            cookie_str.push_str("; HttpOnly");
        }

        cookie_str
    }
}
