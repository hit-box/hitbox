//! Evaluation context for predicates and extractors.
//!
//! [`EvalContext`] is a type-map that allows predicates and extractors to share
//! computed values during a single evaluation phase. This avoids redundant
//! expensive operations (e.g., deserializing a protobuf body into a
//! `DynamicMessage`) when multiple predicates or extractors need the same data.
//!
//! ## Usage
//!
//! ```rust
//! use hitbox_core::EvalContext;
//!
//! struct ParsedProto(String);
//!
//! let mut ctx = EvalContext::new();
//! ctx.insert(ParsedProto("hello".into()));
//!
//! assert!(ctx.contains::<ParsedProto>());
//! assert_eq!(ctx.get::<ParsedProto>().unwrap().0, "hello");
//! ```
//!
//! ## Lifecycle
//!
//! An `EvalContext` is created inside each `cache_policy` implementation:
//! one for the request phase (shared by request predicates and extractors)
//! and another for the response phase (used by response predicates).

use std::any::{Any, TypeId};
use std::collections::HashMap;

/// A type-map for sharing computed values across predicates and extractors.
///
/// Each value is keyed by its concrete type (`TypeId`), so only one value
/// of each type can be stored. Use newtype wrappers to store multiple
/// values of the same underlying type.
///
/// # Thread Safety
///
/// `EvalContext` is `Send + Sync` because all stored values must be
/// `Send + Sync + 'static`. It is passed as `&mut EvalContext` through
/// the evaluation chain, which is always sequential.
pub struct EvalContext {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl EvalContext {
    /// Creates an empty evaluation context.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Inserts a value into the context.
    ///
    /// If a value of this type already exists, it is replaced and the old
    /// value is returned.
    pub fn insert<T: Send + Sync + 'static>(&mut self, val: T) -> Option<T> {
        self.map
            .insert(TypeId::of::<T>(), Box::new(val))
            .and_then(|boxed| boxed.downcast().ok().map(|b| *b))
    }

    /// Returns a reference to a value of the given type, if present.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref())
    }

    /// Returns a mutable reference to a value of the given type, if present.
    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_mut())
    }

    /// Returns a mutable reference to a value of the given type, inserting
    /// a default computed by `f` if not present.
    ///
    /// This is the primary method for expensive lazy initialization:
    ///
    /// ```ignore
    /// let msg = ctx.get_or_insert_with(|| {
    ///     ParsedProto(DynamicMessage::decode(descriptor, body_bytes).unwrap())
    /// });
    /// ```
    pub fn get_or_insert_with<T: Send + Sync + 'static>(
        &mut self,
        f: impl FnOnce() -> T,
    ) -> &mut T {
        self.map
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(f()))
            .downcast_mut()
            .expect("type mismatch in EvalContext (this is a bug)")
    }

    /// Returns `true` if the context contains a value of the given type.
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }

    /// Removes and returns a value of the given type, if present.
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.map
            .remove(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast().ok().map(|b| *b))
    }
}

impl Default for EvalContext {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for EvalContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvalContext")
            .field("entries", &self.map.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StringValue(String);
    struct Counter(u32);

    #[test]
    fn test_insert_and_get() {
        let mut ctx = EvalContext::new();
        ctx.insert(StringValue("hello".into()));

        assert!(ctx.contains::<StringValue>());
        assert_eq!(ctx.get::<StringValue>().unwrap().0, "hello");
    }

    #[test]
    fn test_insert_replaces_and_returns_old() {
        let mut ctx = EvalContext::new();
        assert!(ctx.insert(Counter(1)).is_none());
        let old = ctx.insert(Counter(2));
        assert_eq!(old.unwrap().0, 1);
        assert_eq!(ctx.get::<Counter>().unwrap().0, 2);
    }

    #[test]
    fn test_get_mut() {
        let mut ctx = EvalContext::new();
        ctx.insert(Counter(0));
        ctx.get_mut::<Counter>().unwrap().0 += 1;
        assert_eq!(ctx.get::<Counter>().unwrap().0, 1);
    }

    #[test]
    fn test_get_or_insert_with() {
        let mut ctx = EvalContext::new();

        // First call inserts
        let val = ctx.get_or_insert_with(|| Counter(42));
        assert_eq!(val.0, 42);

        // Second call returns existing
        let val = ctx.get_or_insert_with(|| Counter(99));
        assert_eq!(val.0, 42);
    }

    #[test]
    fn test_remove() {
        let mut ctx = EvalContext::new();
        ctx.insert(Counter(10));
        let removed = ctx.remove::<Counter>();
        assert_eq!(removed.unwrap().0, 10);
        assert!(!ctx.contains::<Counter>());
    }

    #[test]
    fn test_missing_type_returns_none() {
        let ctx = EvalContext::new();
        assert!(ctx.get::<Counter>().is_none());
    }

    #[test]
    fn test_multiple_types() {
        let mut ctx = EvalContext::new();
        ctx.insert(StringValue("a".into()));
        ctx.insert(Counter(1));

        assert_eq!(ctx.get::<StringValue>().unwrap().0, "a");
        assert_eq!(ctx.get::<Counter>().unwrap().0, 1);
    }

    #[test]
    fn test_default() {
        let ctx = EvalContext::default();
        assert!(!ctx.contains::<Counter>());
    }
}
