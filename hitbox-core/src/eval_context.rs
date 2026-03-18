//! Evaluation context for predicates and extractors.
//!
//! [`EvalContext`] is a type-map that allows predicates and extractors to share
//! computed values during a single evaluation phase. This avoids redundant
//! expensive operations (e.g., collecting a chunked body and deserializing it
//! into JSON) when multiple predicates or extractors need the same data.
//!
//! ## Usage
//!
//! ```ignore
//! use hitbox_core::EvalContext;
//!
//! struct ParsedBody(serde_json::Value);
//!
//! let ctx = EvalContext::new();
//!
//! // Async lazy initialization — body is collected once
//! let body = ctx.get_or_insert_with(|| async {
//!     let collected = body.collect().await.unwrap();
//!     ParsedBody(serde_json::from_slice(&collected.data).unwrap())
//! }).await;
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
//! All methods take `&self` and are async, using [`tokio::sync::RwLock`]
//! internally. Values are stored as [`Arc`], so `get` returns `Arc<T>` —
//! no lock guards escape the API. `get_or_insert_with` accepts an async
//! closure, enabling lazy computation of values that require `.await`
//! (like body collection or deserialization).

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use tokio::sync::RwLock;

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
/// [`tokio::sync::RwLock`] and stores values as [`Arc`], so all methods
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
    pub async fn insert<T: Send + Sync + 'static>(&self, val: T) {
        self.map
            .write()
            .await
            .insert(TypeId::of::<T>(), Arc::new(val));
    }

    /// Returns an `Arc` to a value of the given type, if present.
    pub async fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.map
            .read()
            .await
            .get(&TypeId::of::<T>())
            .and_then(|arc| arc.clone().downcast().ok())
    }

    /// Returns an `Arc` to a value of the given type, inserting a default
    /// computed by the async closure `f` if not present.
    ///
    /// Uses double-checked locking: the read lock fast-path avoids write
    /// contention when the value already exists. The computation runs
    /// outside any lock, so it can freely `.await`.
    ///
    /// ```ignore
    /// let body = ctx.get_or_insert_with(|| async {
    ///     let collected = body.collect().await.unwrap();
    ///     ParsedBody(serde_json::from_slice(&collected.data).unwrap())
    /// }).await;
    /// ```
    pub async fn get_or_insert_with<T, F, Fut>(&self, f: F) -> Arc<T>
    where
        T: Send + Sync + 'static,
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        // Fast path: read lock only
        if let Some(val) = self.get::<T>().await {
            return val;
        }
        // Compute outside any lock — can .await freely
        let val = Arc::new(f().await);
        // Write lock only for insertion
        let mut map = self.map.write().await;
        // Double-check: another task may have inserted while we computed
        if let Some(arc) = map.get(&TypeId::of::<T>())
            && let Ok(typed) = arc.clone().downcast::<T>()
        {
            return typed;
        }
        map.insert(TypeId::of::<T>(), val.clone());
        val
    }

    /// Returns `true` if the context contains a value of the given type.
    pub async fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.map.read().await.contains_key(&TypeId::of::<T>())
    }

    /// Removes a value of the given type, returning the `Arc` if present.
    pub async fn remove<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.map
            .write()
            .await
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
        let len = self.map.try_read().map(|m| m.len()).unwrap_or(0);
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

    #[tokio::test]
    async fn test_insert_and_get() {
        let ctx = EvalContext::new();
        ctx.insert(StringValue("hello".into())).await;

        assert!(ctx.contains::<StringValue>().await);
        assert_eq!(ctx.get::<StringValue>().await.unwrap().0, "hello");
    }

    #[tokio::test]
    async fn test_insert_replaces() {
        let ctx = EvalContext::new();
        ctx.insert(Counter(1)).await;
        ctx.insert(Counter(2)).await;
        assert_eq!(ctx.get::<Counter>().await.unwrap().0, 2);
    }

    #[tokio::test]
    async fn test_get_or_insert_with_sync() {
        let ctx = EvalContext::new();

        // First call inserts (sync computation wrapped in async)
        let val = ctx.get_or_insert_with(|| async { Counter(42) }).await;
        assert_eq!(val.0, 42);

        // Second call returns existing
        let val = ctx.get_or_insert_with(|| async { Counter(99) }).await;
        assert_eq!(val.0, 42);
    }

    #[tokio::test]
    async fn test_get_or_insert_with_async() {
        let ctx = EvalContext::new();

        // Simulate async computation (like body collection)
        let val = ctx
            .get_or_insert_with(|| async {
                tokio::task::yield_now().await;
                Counter(42)
            })
            .await;
        assert_eq!(val.0, 42);

        // Second call returns cached value, does not compute
        let val = ctx
            .get_or_insert_with(|| async { panic!("should not be called") as Counter })
            .await;
        assert_eq!(val.0, 42);
    }

    #[tokio::test]
    async fn test_remove() {
        let ctx = EvalContext::new();
        ctx.insert(Counter(10)).await;
        let removed = ctx.remove::<Counter>().await;
        assert_eq!(removed.unwrap().0, 10);
        assert!(!ctx.contains::<Counter>().await);
    }

    #[tokio::test]
    async fn test_missing_type_returns_none() {
        let ctx = EvalContext::new();
        assert!(ctx.get::<Counter>().await.is_none());
    }

    #[tokio::test]
    async fn test_multiple_types() {
        let ctx = EvalContext::new();
        ctx.insert(StringValue("a".into())).await;
        ctx.insert(Counter(1)).await;

        assert_eq!(ctx.get::<StringValue>().await.unwrap().0, "a");
        assert_eq!(ctx.get::<Counter>().await.unwrap().0, 1);
    }

    #[tokio::test]
    async fn test_default() {
        let ctx = EvalContext::default();
        assert!(!ctx.contains::<Counter>().await);
    }

    #[tokio::test]
    async fn test_arc_is_send_across_tasks() {
        let ctx = Arc::new(EvalContext::new());
        ctx.insert(Counter(42)).await;

        let ctx2 = ctx.clone();
        let handle = tokio::spawn(async move {
            let val = ctx2.get::<Counter>().await.unwrap();
            assert_eq!(val.0, 42);
        });
        handle.await.unwrap();
    }
}
