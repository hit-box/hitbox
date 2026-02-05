//! Tests for CompositionPolicy builder pattern.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use hitbox_backend::composition::CompositionPolicy;
use hitbox_backend::composition::policy::{
    OptimisticParallelWritePolicy, ParallelReadPolicy, RaceReadPolicy, RefillPolicy,
    SequentialReadPolicy, SequentialWritePolicy,
};
use hitbox_backend::format::{Format, JsonFormat};
use hitbox_backend::{
    Backend, BackendResult, CacheBackend, CacheKeyFormat, CompositionBackend, Compressor,
    DeleteStatus, PassthroughCompressor,
};
use hitbox_core::{
    BoxContext, CacheContext, CacheKey, CacheValue, CacheableResponse, EntityPolicyConfig,
    Predicate, Raw,
};
use serde::{Deserialize, Serialize};

use crate::common::TestOffloadManager;

#[cfg(feature = "rkyv_format")]
use rkyv::{Archive, Serialize as RkyvSerialize};

/// Simple in-memory backend for testing
#[derive(Clone)]
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
        let existed = self.store.lock().unwrap().remove(key).is_some();
        Ok(if existed {
            DeleteStatus::Deleted(1)
        } else {
            DeleteStatus::Missing
        })
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(
    feature = "rkyv_format",
    derive(Archive, RkyvSerialize, rkyv::Deserialize)
)]
struct TestValue {
    data: String,
}

impl CacheableResponse for TestValue {
    type Cached = Self;
    type Subject = Self;
    type IntoCachedFuture = std::future::Ready<hitbox_core::CachePolicy<Self::Cached, Self>>;
    type FromCachedFuture = std::future::Ready<Self>;

    async fn cache_policy<P: Predicate<Subject = Self::Subject> + Send + Sync>(
        self,
        _predicate: P,
        _config: &EntityPolicyConfig,
    ) -> hitbox_core::ResponseCachePolicy<Self> {
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
async fn test_composition_policy_default() {
    let policy = CompositionPolicy::new();

    // Default policies should be set
    let _ = policy.read_policy(); // SequentialReadPolicy
    let _ = policy.write_policy(); // OptimisticParallelWritePolicy
    let _ = policy.refill_policy(); // RefillPolicy::Never (default)
}

#[tokio::test]
async fn test_composition_policy_with_read() {
    let policy = CompositionPolicy::new().read(RaceReadPolicy::new());

    // Should have RaceReadPolicy
    let _ = policy.read_policy();
}

#[tokio::test]
async fn test_composition_policy_with_write() {
    let policy = CompositionPolicy::new().write(SequentialWritePolicy::new());

    // Should have SequentialWritePolicy
    let _ = policy.write_policy();
}

#[tokio::test]
async fn test_composition_policy_with_refill() {
    let policy = CompositionPolicy::new().refill(RefillPolicy::Never);

    // Should have RefillPolicy::Never
    let _ = policy.refill_policy();
}

#[tokio::test]
async fn test_composition_policy_chained() {
    let policy = CompositionPolicy::new()
        .read(ParallelReadPolicy::new())
        .write(SequentialWritePolicy::new())
        .refill(RefillPolicy::Never);

    // All custom policies should be set
    let _ = policy.read_policy();
    let _ = policy.write_policy();
    let _ = policy.refill_policy();
}

#[tokio::test]
async fn test_backend_with_composition_policy() {
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();

    let policy = CompositionPolicy::new()
        .read(RaceReadPolicy::new())
        .write(SequentialWritePolicy::new())
        .refill(RefillPolicy::Never);

    let backend = CompositionBackend::new(l1, l2, TestOffloadManager).with_policy(policy);

    // All custom policies should be set
    let _ = backend.read_policy();
    let _ = backend.write_policy();
    let _ = backend.refill_policy();
}

#[tokio::test]
async fn test_backend_with_policy_functional() {
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();

    let policy = CompositionPolicy::new()
        .read(SequentialReadPolicy::new())
        .write(OptimisticParallelWritePolicy::new())
        .refill(RefillPolicy::Always);

    let backend =
        CompositionBackend::new(l1.clone(), l2.clone(), TestOffloadManager).with_policy(policy);

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(
        TestValue {
            data: "test_value".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    let mut ctx: BoxContext = CacheContext::default().boxed();

    // Write via backend
    backend
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    // Read via backend
    let result = backend.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().data, "test_value");
}
