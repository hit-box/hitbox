//! Tests for context-based operations with dynamic dispatch (trait objects).
//!
//! Note: Refill tests have been moved to hitbox crate where CacheFuture handles refill.

use std::future::Future;
use std::sync::Arc;

use chrono::Utc;
use hitbox_backend::composition::CompositionBackend;
use hitbox_backend::{CacheBackend, SyncBackend};
use hitbox_core::{
    BoxContext, CacheContext, CacheKey, CacheValue, CacheableResponse, EntityPolicyConfig, Offload,
    Predicate,
};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// Test offload that spawns tasks with tokio::spawn
#[derive(Clone, Debug)]
struct TestOffload;

impl Offload<'static> for TestOffload {
    fn spawn<F>(&self, _kind: impl Into<SmolStr>, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(future);
    }

    fn spawn_with_key<F>(&self, _key: CacheKey, kind: impl Into<SmolStr>, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.spawn(kind, future);
    }
}

#[cfg(feature = "rkyv_format")]
use rkyv::{Archive, Serialize as RkyvSerialize};

use crate::common::TestBackend;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
async fn test_direct_write_through_trait_object() {
    // Create backends
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();

    // Clone for inspection
    let l1_inspect = l1.clone();
    let l2_inspect = l2.clone();

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(
        TestValue {
            data: "direct_write".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Create composition as trait object
    let composition = CompositionBackend::new(l1, l2, TestOffload);
    let backend: Arc<SyncBackend> = Arc::new(composition);

    // Direct write through trait object
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    // Verify both L1 and L2 have the data
    assert!(
        l1_inspect.has(&key),
        "L1 should have data after direct write"
    );
    assert!(
        l2_inspect.has(&key),
        "L2 should have data after direct write"
    );
}
