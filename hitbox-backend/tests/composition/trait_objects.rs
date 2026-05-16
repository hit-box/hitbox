use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use hitbox_backend::format::{Format, JsonFormat};
use hitbox_backend::{
    Backend, BackendResult, CacheBackend, CacheKeyFormat, CompositionBackend, Compressor,
    DeleteStatus, PassthroughCompressor, SyncBackend,
};
use hitbox_core::tag::{CacheTag, TagExtractor};
use hitbox_core::{
    BoxContext, CacheContext, CacheKey, CacheValue, CacheableResponse, EntityPolicyConfig,
    Predicate, Raw, ResponseCachePolicy,
};
use serde::{Deserialize, Serialize};

use crate::common::TestOffloadManager;

#[cfg(feature = "rkyv_format")]
use rkyv::{Archive, Serialize as RkyvSerialize};

// Simple in-memory backend for testing
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

    async fn cache_policy<P, TE>(
        self,
        _predicate: P,
        _tag_extractor: Option<TE>,
        _config: &EntityPolicyConfig,
    ) -> (ResponseCachePolicy<Self>, Vec<CacheTag>)
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
async fn test_boxed_composition_backend() {
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let composition = CompositionBackend::new(l1, l2, TestOffloadManager);

    // Box the CompositionBackend itself
    let boxed: Box<CompositionBackend<_, _, _>> = Box::new(composition);

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(
        TestValue {
            data: "test_value".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Should work through Box
    let mut ctx: BoxContext = CacheContext::default().boxed();
    boxed
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    let result = boxed.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().data, "test_value");
}

#[tokio::test]
async fn test_arc_composition_backend() {
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let composition = CompositionBackend::new(l1, l2, TestOffloadManager);

    // Arc the CompositionBackend itself
    let arc: Arc<CompositionBackend<_, _, _>> = Arc::new(composition);

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(
        TestValue {
            data: "test_value".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Should work through Arc
    let mut ctx: BoxContext = CacheContext::default().boxed();
    arc.set::<TestValue>(&key, &value, &mut ctx).await.unwrap();

    let result = arc.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().data, "test_value");

    // Arc should be cloneable
    let arc2 = arc.clone();
    let result2 = arc2.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result2.is_some());
}

#[tokio::test]
async fn test_ref_composition_backend() {
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let composition = CompositionBackend::new(l1, l2, TestOffloadManager);

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(
        TestValue {
            data: "test_value".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Should work through reference
    let mut ctx: BoxContext = CacheContext::default().boxed();
    composition
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    let result = composition.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().data, "test_value");
}

#[tokio::test]
async fn test_composition_as_dyn_backend() {
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let composition = CompositionBackend::new(l1, l2, TestOffloadManager);

    // Use CompositionBackend as trait object
    let backend: &dyn Backend = &composition;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(
        TestValue {
            data: "test_value".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Should work through trait object
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    let result = backend.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().data, "test_value");
}

#[tokio::test]
async fn test_arc_composition_as_dyn_backend() {
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let composition = CompositionBackend::new(l1, l2, TestOffloadManager);

    // Arc CompositionBackend and use as trait object
    let backend: Arc<SyncBackend> = Arc::new(composition);

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(
        TestValue {
            data: "test_value".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Should work through Arc trait object
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    let result = backend.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().data, "test_value");
}

#[tokio::test]
async fn test_arc_sync_composition_as_dyn_backend() {
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let composition = CompositionBackend::new(l1, l2, TestOffloadManager);

    // Arc CompositionBackend and use as SyncBackend trait object
    let backend: Arc<SyncBackend> = Arc::new(composition);

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(
        TestValue {
            data: "test_value".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Should work through Arc'd trait object
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    let result = backend.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().data, "test_value");

    // Arc trait object should be cloneable
    let backend2 = backend.clone();
    let result2 = backend2.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result2.is_some());
}
