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
//! let ctx = EvalContext::new();
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
//!
//! ## Interior Mutability
//!
//! All methods take `&self`, using a [`std::sync::RwLock`] internally.
//! Values are stored as [`Arc`], so `get` returns `Arc<T>` — the lock is
//! held only for the duration of a HashMap lookup and an `Arc::clone`,
//! never across `.await` points. This makes `EvalContext` safe to share
//! via `&EvalContext` across concurrent `tokio::spawn` tasks.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A type-map for sharing computed values across predicates and extractors.
///
/// Each value is keyed by its concrete type (`TypeId`), so only one value
/// of each type can be stored. Use newtype wrappers to store multiple
/// values of the same underlying type.
///
/// # Thread Safety
///
/// `EvalContext` is `Send + Sync` because all stored values must be
/// `Send + Sync + 'static`. It uses interior mutability via
/// [`std::sync::RwLock`] and stores values as [`Arc`], so all methods
/// take `&self` and return owned `Arc<T>` handles — no lock guards
/// escape the API.
pub struct EvalContext {
    map: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl EvalContext {
    /// Creates an empty evaluation context.
    pub fn new() -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
        }
    }

    /// Inserts a value into the context.
    ///
    /// If a value of this type already exists, it is replaced.
    pub fn insert<T: Send + Sync + 'static>(&self, val: T) {
        self.map
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(TypeId::of::<T>(), Arc::new(val));
    }

    /// Returns an `Arc` to a value of the given type, if present.
    ///
    /// The internal lock is held only for the HashMap lookup and `Arc::clone`,
    /// then released immediately.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.map
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&TypeId::of::<T>())
            .and_then(|arc| arc.clone().downcast().ok())
    }

    /// Returns an `Arc` to a value of the given type, inserting a default
    /// computed by `f` if not present.
    ///
    /// Uses double-checked locking: the read lock fast-path avoids write
    /// contention when the value already exists.
    ///
    /// ```ignore
    /// let msg = ctx.get_or_insert_with(|| {
    ///     ParsedProto(DynamicMessage::decode(descriptor, body_bytes).unwrap())
    /// });
    /// ```
    pub fn get_or_insert_with<T: Send + Sync + 'static>(&self, f: impl FnOnce() -> T) -> Arc<T> {
        // Fast path: read lock only
        if let Some(val) = self.get::<T>() {
            return val;
        }
        // Compute outside any lock
        let val = Arc::new(f());
        // Write lock only for insertion
        let mut map = self.map.write().unwrap_or_else(|e| e.into_inner());
        // Double-check: another thread may have inserted while we computed
        if let Some(arc) = map.get(&TypeId::of::<T>())
            && let Ok(typed) = arc.clone().downcast::<T>()
        {
            return typed;
        }
        map.insert(TypeId::of::<T>(), val.clone());
        val
    }

    /// Returns `true` if the context contains a value of the given type.
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.map
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&TypeId::of::<T>())
    }

    /// Removes a value of the given type, returning the `Arc` if present.
    pub fn remove<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.map
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&TypeId::of::<T>())
            .and_then(|arc| arc.downcast().ok())
    }
}

impl Default for EvalContext {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for EvalContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let len = self.map.read().map(|m| m.len()).unwrap_or(0);
        f.debug_struct("EvalContext")
            .field("entries", &len)
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
        let ctx = EvalContext::new();
        ctx.insert(StringValue("hello".into()));

        assert!(ctx.contains::<StringValue>());
        assert_eq!(ctx.get::<StringValue>().unwrap().0, "hello");
    }

    #[test]
    fn test_insert_replaces() {
        let ctx = EvalContext::new();
        ctx.insert(Counter(1));
        ctx.insert(Counter(2));
        assert_eq!(ctx.get::<Counter>().unwrap().0, 2);
    }

    #[test]
    fn test_get_or_insert_with() {
        let ctx = EvalContext::new();

        // First call inserts
        let val = ctx.get_or_insert_with(|| Counter(42));
        assert_eq!(val.0, 42);

        // Second call returns existing
        let val = ctx.get_or_insert_with(|| Counter(99));
        assert_eq!(val.0, 42);
    }

    #[test]
    fn test_remove() {
        let ctx = EvalContext::new();
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
        let ctx = EvalContext::new();
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

    #[test]
    fn test_arc_is_send_across_threads() {
        let ctx = Arc::new(EvalContext::new());
        ctx.insert(Counter(42));

        let ctx2 = ctx.clone();
        let handle = std::thread::spawn(move || {
            let val = ctx2.get::<Counter>().unwrap();
            assert_eq!(val.0, 42);
        });
        handle.join().unwrap();
    }
}
