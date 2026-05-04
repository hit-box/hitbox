//! Trait for composing backends into layered cache hierarchies.
//!
//! The `Compose` trait provides a fluent API for building `CompositionBackend` instances,
//! making it easy to create multi-level cache hierarchies with custom policies.
//!
//! # Examples
//!
//! Basic composition with default policies:
//! ```ignore
//! use hitbox_backend::composition::Compose;
//!
//! let cache = mem_backend.compose(redis_backend, offload);
//! ```
//!
//! Composition with custom policies:
//! ```ignore
//! use hitbox_backend::composition::{Compose, CompositionPolicy};
//! use hitbox_backend::composition::policy::{RaceReadPolicy, SequentialWritePolicy};
//!
//! let policy = CompositionPolicy::new()
//!     .read(RaceReadPolicy::new())
//!     .write(SequentialWritePolicy::new());
//!
//! let cache = mem_backend.compose_with(redis_backend, offload, policy);
//! ```

use super::policy::{CompositionReadPolicy, CompositionWritePolicy};
use super::{CompositionBackend, CompositionPolicy};
use crate::Backend;
use hitbox_core::Offload;

/// Trait for composing backends into layered cache hierarchies.
///
/// This trait is automatically implemented for all types that implement `Backend`,
/// providing a fluent API for creating `CompositionBackend` instances.
///
/// # Examples
///
/// ```ignore
/// use hitbox_backend::composition::Compose;
///
/// // Simple composition with default policies
/// let cache = l1_backend.compose(l2_backend, offload);
///
/// // Composition with custom policies
/// let policy = CompositionPolicy::new()
///     .read(RaceReadPolicy::new())
///     .write(SequentialWritePolicy::new());
///
/// let cache = l1_backend.compose_with(l2_backend, offload, policy);
/// ```
pub trait Compose: Backend + Sized {
    /// Compose this backend with another backend as L2, using default policies.
    ///
    /// This creates a `CompositionBackend` where:
    /// - `self` becomes L1 (first layer, checked first on reads)
    /// - `l2` becomes L2 (second layer, checked if L1 misses)
    ///
    /// Default policies:
    /// - Read: `SequentialReadPolicy` (try L1 first, then L2)
    /// - Write: `OptimisticParallelWritePolicy` (write to both, succeed if ≥1 succeeds)
    /// - Refill: `AlwaysRefill` (always populate L1 after L2 hit)
    ///
    /// # Arguments
    /// * `l2` - The second-layer backend
    /// * `offload` - Offload manager for background tasks
    ///
    /// # Example
    /// ```ignore
    /// use hitbox_backend::composition::Compose;
    /// use hitbox_moka::MokaBackend;
    /// use hitbox_redis::{RedisBackend, ConnectionMode};
    ///
    /// let moka = MokaBackend::builder().max_entries(1000).build();
    /// let redis = RedisBackend::builder()
    ///     .connection(ConnectionMode::single("redis://localhost/"))
    ///     .build()?;
    ///
    /// // Moka as L1, Redis as L2
    /// let cache = moka.compose(redis, offload);
    /// ```
    fn compose<L2, O>(self, l2: L2, offload: O) -> CompositionBackend<Self, L2, O>
    where
        L2: Backend,
        O: Offload<'static>,
    {
        CompositionBackend::new(self, l2, offload)
    }

    /// Compose this backend with another backend as L2, using custom policies.
    ///
    /// This provides full control over read, write, and refill policies.
    ///
    /// # Arguments
    /// * `l2` - The second-layer backend
    /// * `offload` - Offload manager for background tasks
    /// * `policy` - Custom composition policies
    ///
    /// # Example
    /// ```ignore
    /// use hitbox_backend::composition::{Compose, CompositionPolicy};
    /// use hitbox_backend::composition::policy::{RaceReadPolicy, SequentialWritePolicy};
    ///
    /// let policy = CompositionPolicy::new()
    ///     .read(RaceReadPolicy::new())
    ///     .write(SequentialWritePolicy::new());
    ///
    /// let cache = l1.compose_with(l2, offload, policy);
    /// ```
    fn compose_with<L2, O, R, W>(
        self,
        l2: L2,
        offload: O,
        policy: CompositionPolicy<R, W>,
    ) -> CompositionBackend<Self, L2, O, R, W>
    where
        L2: Backend,
        O: Offload<'static>,
        R: CompositionReadPolicy,
        W: CompositionWritePolicy,
    {
        CompositionBackend::new(self, l2, offload).with_policy(policy)
    }
}

// Blanket implementation for all Backend types
impl<T: Backend> Compose for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{Format, JsonFormat};
    use crate::{
        Backend, BackendResult, CacheBackend, CacheKeyFormat, Compressor, DeleteStatus,
        PassthroughCompressor,
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use hitbox_core::tag::{CacheTag, TagExtractor};
    use hitbox_core::{
        BoxContext, CacheContext, CacheKey, CachePolicy, CacheValue, CacheableResponse,
        EntityPolicyConfig, Predicate, Raw,
    };
    use serde::{Deserialize, Serialize};
    use smol_str::SmolStr;
    use std::collections::HashMap;
    use std::future::Future;
    use std::sync::{Arc, Mutex};

    #[cfg(feature = "rkyv_format")]
    use rkyv::{Archive, Serialize as RkyvSerialize};

    /// Test offload that spawns tasks with tokio::spawn
    #[derive(Clone, Debug)]
    struct TestOffload;

    impl Offload<'static> for TestOffload {
        #[allow(deprecated)]
        fn spawn<F>(&self, _kind: impl Into<SmolStr>, future: F)
        where
            F: Future<Output = ()> + Send + 'static,
        {
            tokio::spawn(future);
        }
    }

    #[derive(Clone, Debug)]
    struct TestBackend {
        store: Arc<Mutex<HashMap<CacheKey, CacheValue<Raw>>>>,
    }

    impl TestBackend {
        fn new() -> Self {
            Self {
                store: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl Backend for TestBackend {
        async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
            Ok(self.store.lock().unwrap().get(key).cloned())
        }

        async fn write(&self, key: &CacheKey, value: CacheValue<Raw>) -> BackendResult<()> {
            self.store.lock().unwrap().insert(key.clone(), value);
            Ok(())
        }

        async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
            match self.store.lock().unwrap().remove(key) {
                Some(_) => Ok(DeleteStatus::Deleted(1)),
                None => Ok(DeleteStatus::Missing),
            }
        }

        fn value_format(&self) -> &dyn Format {
            &JsonFormat
        }

        fn key_format(&self) -> &CacheKeyFormat {
            &CacheKeyFormat::Bitcode
        }

        fn compressor(&self) -> &dyn Compressor {
            &PassthroughCompressor
        }
    }

    impl CacheBackend for TestBackend {}

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[cfg_attr(
        feature = "rkyv_format",
        derive(Archive, RkyvSerialize, rkyv::Deserialize)
    )]
    struct CachedData {
        value: String,
    }

    struct MockResponse;

    impl CacheableResponse for MockResponse {
        type Cached = CachedData;
        type Subject = MockResponse;
        type IntoCachedFuture = std::future::Ready<CachePolicy<Self::Cached, Self>>;
        type FromCachedFuture = std::future::Ready<Self>;

        async fn cache_policy<P, TE>(
            self,
            _predicate: P,
            _tag_extractor: TE,
            _config: &EntityPolicyConfig,
        ) -> (CachePolicy<CacheValue<Self::Cached>, Self>, Vec<CacheTag>)
        where
            P: Predicate<Subject = Self::Subject> + Send + Sync,
            TE: TagExtractor<Subject = Self::Subject> + Send + Sync,
        {
            unimplemented!()
        }

        fn into_cached(self) -> Self::IntoCachedFuture {
            unimplemented!()
        }

        fn from_cached(_cached: Self::Cached) -> Self::FromCachedFuture {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_compose_basic() {
        let l1 = TestBackend::new();
        let l2 = TestBackend::new();
        let offload = TestOffload;

        // Use compose trait
        let cache = l1.clone().compose(l2.clone(), offload);

        let key = CacheKey::from_str("test", "key1");
        let value = CacheValue::new(
            CachedData {
                value: "test_value".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
        );

        // Write and read
        let mut ctx: BoxContext = CacheContext::default().boxed();
        cache
            .set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = cache.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(result.unwrap().data().value, "test_value");

        // Verify both layers have the data
        let mut ctx: BoxContext = CacheContext::default().boxed();
        assert!(
            l1.get::<MockResponse>(&key, &mut ctx)
                .await
                .unwrap()
                .is_some()
        );
        let mut ctx: BoxContext = CacheContext::default().boxed();
        assert!(
            l2.get::<MockResponse>(&key, &mut ctx)
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn test_compose_with_policy() {
        use super::super::policy::{RaceReadPolicy, RefillPolicy};

        let l1 = TestBackend::new();
        let l2 = TestBackend::new();
        let offload = TestOffload;

        // Use compose_with to specify custom policies
        let policy = CompositionPolicy::new()
            .read(RaceReadPolicy::new())
            .refill(RefillPolicy::Never);

        let cache = l1.clone().compose_with(l2.clone(), offload, policy);

        let key = CacheKey::from_str("test", "key1");
        let value = CacheValue::new(
            CachedData {
                value: "from_l2".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
        );

        // Populate only L2
        let mut ctx: BoxContext = CacheContext::default().boxed();
        l2.set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        // Read through composition (should use RaceReadPolicy)
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = cache.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(result.unwrap().data().value, "from_l2");

        // With NeverRefill, L1 should NOT be populated
        let mut ctx: BoxContext = CacheContext::default().boxed();
        assert!(
            l1.get::<MockResponse>(&key, &mut ctx)
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn test_compose_nested() {
        // Test that composed backends can be further composed
        let l1 = TestBackend::new();
        let l2 = TestBackend::new();
        let l3 = TestBackend::new();
        let offload = TestOffload;

        // Create L2+L3 composition
        let l2_l3 = l2.clone().compose(l3.clone(), offload.clone());

        // Compose L1 with the (L2+L3) composition
        let cache = l1.clone().compose(l2_l3, offload);

        let key = CacheKey::from_str("test", "nested_key");
        let value = CacheValue::new(
            CachedData {
                value: "nested_value".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
        );

        // Write through nested composition
        let mut ctx: BoxContext = CacheContext::default().boxed();
        cache
            .set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        // All three levels should have the data
        let mut ctx: BoxContext = CacheContext::default().boxed();
        assert!(
            l1.get::<MockResponse>(&key, &mut ctx)
                .await
                .unwrap()
                .is_some()
        );
        let mut ctx: BoxContext = CacheContext::default().boxed();
        assert!(
            l2.get::<MockResponse>(&key, &mut ctx)
                .await
                .unwrap()
                .is_some()
        );
        let mut ctx: BoxContext = CacheContext::default().boxed();
        assert!(
            l3.get::<MockResponse>(&key, &mut ctx)
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn test_compose_chaining() {
        use super::super::policy::RaceReadPolicy;

        let l1 = TestBackend::new();
        let l2 = TestBackend::new();
        let offload = TestOffload;

        // Test method chaining: compose + builder methods
        let cache = l1
            .clone()
            .compose(l2.clone(), offload)
            .read(RaceReadPolicy::new());

        let key = CacheKey::from_str("test", "chain_key");
        let value = CacheValue::new(
            CachedData {
                value: "chain_value".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
        );

        let mut ctx: BoxContext = CacheContext::default().boxed();
        cache
            .set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = cache.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(result.unwrap().data().value, "chain_value");
    }
}
