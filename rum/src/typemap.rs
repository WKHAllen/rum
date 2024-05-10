//! A type map interface.

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// A type map. Internally, this does the heavy lifting for the state manager.
#[derive(Debug, Default)]
pub struct TypeMap(HashMap<TypeId, Box<dyn Any + Send + Sync>>);

#[allow(dead_code)]
impl TypeMap {
    /// Creates a new empty type map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a new value into the map. If a value of the same type is already
    /// in the map, it will be removed and returned.
    pub fn insert<T>(&mut self, value: T) -> Option<T>
    where
        T: Send + Sync + 'static,
    {
        self.0
            .insert(value.type_id(), Box::new(value))
            .map(|x| *x.downcast().unwrap())
    }

    /// Gets an immutable reference to a value in the map.
    pub fn get<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.0
            .get(&TypeId::of::<T>())
            .map(|x| x.downcast_ref().unwrap())
    }

    /// Gets a mutable reference to a value in the map.
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.0
            .get_mut(&TypeId::of::<T>())
            .map(|x| x.downcast_mut().unwrap())
    }

    /// Gets a clone of a value in the map.
    pub fn get_cloned<T>(&self) -> Option<T>
    where
        T: Clone + 'static,
    {
        self.get().cloned()
    }

    /// Gets a copy of a value in the map.
    pub fn get_copied<T>(&self) -> Option<T>
    where
        T: Copy + 'static,
    {
        self.get().copied()
    }

    /// Removes and returns a value in the map.
    pub fn remove<T>(&mut self) -> Option<T>
    where
        T: 'static,
    {
        self.0
            .remove(&TypeId::of::<T>())
            .map(|x| *x.downcast().unwrap())
    }
}
