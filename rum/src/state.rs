//! State management types and extractors.

use crate::typemap::TypeMap;
use std::borrow::{Borrow, BorrowMut};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// An extractor for a value of type `T` stored in the state management system.
/// `T` must implement `Clone` in order to be used as a state. This usually
/// means you'll want to wrap your data in an `Arc`. This `deref`s to `T`, and
/// can be moved out of `self` with [`into_inner`](Self::into_inner).
pub struct State<T>(pub T)
where
    T: Clone;

impl<T> State<T>
where
    T: Clone,
{
    /// Moves `T` out of this wrapper.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for State<T>
where
    T: Clone,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for State<T>
where
    T: Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Borrow<T> for State<T>
where
    T: Clone,
{
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T> BorrowMut<T> for State<T>
where
    T: Clone,
{
    fn borrow_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// The global state management system.
#[derive(Debug, Clone, Default)]
pub(crate) struct StateManager(pub Arc<TypeMap>);

impl Deref for StateManager {
    type Target = TypeMap;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<TypeMap> for StateManager {
    fn borrow(&self) -> &TypeMap {
        &self.0
    }
}
