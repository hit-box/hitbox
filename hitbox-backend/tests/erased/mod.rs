use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use hitbox_backend::{
    Backend, BackendResult, CacheBackend, CacheKeyFormat, CompositionBackend, SyncBackend,
};
use hitbox_core::tag::{CacheTag, TagExtractor};
use hitbox_core::{
    BoxContext, CacheContext, CacheKey, CacheValue, CacheableResponse, EntityPolicyConfig,
    Predicate, Raw, ResponseCachePolicy,
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::common::TestOffloadManager;

#[cfg(feature = "rkyv_format")]
use rkyv::{Archive, Serialize as RkyvSerialize};

#[derive(Debug, Clone)]
struct MemBackend {
    storage: Arc<RwLock<HashMap<String, Raw>>>,
}

impl MemBackend {
    fn new() -> Self {
        let mut storage = HashMap::new();
        // URL-encoded format: key1=
        storage.insert(
            "key1=".to_owned(),
            bytes::Bytes::from(&b"{\"name\": \"test\", \"index\": 42}"[..]),
        );
        MemBackend {
            storage: Arc::new(RwLock::new(storage)),
        }
    }
}

#[async_trait]
impl Backend for MemBackend {
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
        let lock = self.storage.read().await;
        let key_str = String::from_utf8(CacheKeyFormat::UrlEncoded.serialize(key)?).unwrap();
        let value = lock.get(&key_str).cloned();
        Ok(value.map(|value| CacheValue::new(value, Some(Utc::now()), Some(Utc::now()))))
    }

    async fn write(&self, key: &CacheKey, value: CacheValue<Raw>) -> BackendResult<()> {
        let mut lock = self.storage.write().await;
        let key_str = String::from_utf8(CacheKeyFormat::UrlEncoded.serialize(key)?).unwrap();
        lock.insert(key_str, value.data().clone());
        Ok(())
    }

    async fn remove(&self, _key: &CacheKey) -> BackendResult<hitbox_backend::DeleteStatus> {
        todo!()
    }
}

impl CacheBackend for MemBackend {}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(
    feature = "rkyv_format",
    derive(Archive, RkyvSerialize, rkyv::Deserialize)
)]
struct Value {
    name: String,
    index: u8,
}

impl CacheableResponse for Value {
    type Cached = Self;
    type Subject = Self;
    type IntoCachedFuture = std::future::Ready<hitbox_core::CachePolicy<Self::Cached, Self>>;
    type FromCachedFuture = std::future::Ready<Self>;

    async fn cache_policy<P, TE>(
        self,
        _predicates: P,
        _tag_extractor: TE,
        _: &EntityPolicyConfig,
    ) -> (ResponseCachePolicy<Self>, Vec<CacheTag>)
    where
        P: Predicate<Subject = Self::Subject> + Send + Sync,
        TE: TagExtractor<Subject = Self::Subject> + Send + Sync,
    {
        todo!()
    }

    fn into_cached(self) -> Self::IntoCachedFuture {
        todo!()
    }

    fn from_cached(_cached: Self::Cached) -> Self::FromCachedFuture {
        todo!()
    }
}

struct Cache<B> {
    backend: B,
}

impl<B> Cache<B>
where
    B: CacheBackend + Sync,
{
    fn new(backend: B) -> Self {
        Cache { backend }
    }

    async fn test(&self) {
        let value = CacheValue::new(
            Value {
                name: "value3".to_owned(),
                index: 128,
            },
            Some(Utc::now()),
            Some(Utc::now()),
        );
        let mut ctx: BoxContext = CacheContext::default().boxed();
        self.backend
            .set::<Value>(&CacheKey::from_str("key3", ""), &value, &mut ctx)
            .await
            .unwrap();
        dbg!(
            self.backend
                .get::<Value>(&CacheKey::from_str("key3", ""), &mut ctx)
                .await
                .unwrap()
        );
    }
}

#[tokio::test]
async fn dyn_backend() {
    let key1 = CacheKey::from_str("key1", "");
    let key2 = CacheKey::from_str("key2", "");
    let storage = MemBackend::new();
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let value = storage.get::<Value>(&key1, &mut ctx).await.unwrap();
    dbg!(value);

    let backend: Box<dyn Backend> = Box::new(storage);
    let value = backend.get::<Value>(&key1, &mut ctx).await.unwrap();
    dbg!(value);

    let value = CacheValue::new(
        Value {
            name: "value2".to_owned(),
            index: 255,
        },
        Some(Utc::now()),
        Some(Utc::now()),
    );
    backend.set::<Value>(&key2, &value, &mut ctx).await.unwrap();
    let value = backend.get::<Value>(&key2, &mut ctx).await.unwrap();
    dbg!(value);

    let cache = Cache::new(backend);
    cache.test().await;

    let cache = Cache::new(MemBackend::new());
    cache.test().await;
}

#[tokio::test]
async fn test_composition_with_cloneable_backends() {
    // Create two separate cloneable backends
    let l1 = MemBackend::new();
    let l2 = MemBackend::new();

    // Compose them
    let composition = CompositionBackend::new(l1, l2, TestOffloadManager);

    // Test 1: Write through composition - should populate both layers
    let key_both = CacheKey::from_str("key_both", "");
    let value_both = CacheValue::new(
        Value {
            name: "both_layers".to_owned(),
            index: 1,
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );
    let mut ctx: BoxContext = CacheContext::default().boxed();
    composition
        .set::<Value>(&key_both, &value_both, &mut ctx)
        .await
        .unwrap();

    // Read should return the value
    let result = composition.get::<Value>(&key_both, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().name, "both_layers");

    // Test 2: Key that doesn't exist - should return None
    let key_missing = CacheKey::from_str("missing", "");
    let result = composition
        .get::<Value>(&key_missing, &mut ctx)
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_composition_with_arc_dyn_backends() {
    // Create two separate backends with Arc<SyncBackend>
    // This works because there's an impl Backend for Arc<SyncBackend>
    let l1: Arc<SyncBackend> = Arc::new(MemBackend::new());
    let l2: Arc<SyncBackend> = Arc::new(MemBackend::new());

    // Compose them
    let composition = CompositionBackend::new(l1, l2, TestOffloadManager);

    // Write a value
    let key = CacheKey::from_str("arc_key", "");
    let value = CacheValue::new(
        Value {
            name: "arc_value".to_owned(),
            index: 42,
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );
    let mut ctx: BoxContext = CacheContext::default().boxed();
    composition
        .set::<Value>(&key, &value, &mut ctx)
        .await
        .unwrap();

    // Read it back
    let result = composition.get::<Value>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().name, "arc_value");
}

#[tokio::test]
async fn test_composition_l1_l2_different_keys() {
    // Create L1 and L2 backends
    let l1_mem = MemBackend::new();
    let l2_mem = MemBackend::new();

    let mut ctx: BoxContext = CacheContext::default().boxed();

    // Populate L1 with key1
    let key1 = CacheKey::from_str("key1", "");
    let value1 = CacheValue::new(
        Value {
            name: "l1_only".to_owned(),
            index: 10,
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );
    l1_mem.set::<Value>(&key1, &value1, &mut ctx).await.unwrap();

    // Populate L2 with key2
    let key2 = CacheKey::from_str("key2", "");
    let value2 = CacheValue::new(
        Value {
            name: "l2_only".to_owned(),
            index: 20,
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );
    l2_mem.set::<Value>(&key2, &value2, &mut ctx).await.unwrap();

    // Create composition with cloneable backends
    let composition = CompositionBackend::new(l1_mem, l2_mem, TestOffloadManager);

    // Test 1: Read key1 - should hit L1
    let result = composition.get::<Value>(&key1, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().name, "l1_only");

    // Test 2: Read key2 - should miss L1, hit L2
    let result = composition.get::<Value>(&key2, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().name, "l2_only");

    // Test 3: Read key3 - should miss both
    let key3 = CacheKey::from_str("key3", "");
    let result = composition.get::<Value>(&key3, &mut ctx).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_composition_backend_as_trait_object() {
    // Create L1 and L2 backends
    let l1_mem = MemBackend::new();
    let l2_mem = MemBackend::new();

    let mut ctx: BoxContext = CacheContext::default().boxed();

    // Populate L1 with one key
    let key_l1 = CacheKey::from_str("l1_key", "");
    let value_l1 = CacheValue::new(
        Value {
            name: "in_l1".to_owned(),
            index: 11,
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );
    l1_mem
        .set::<Value>(&key_l1, &value_l1, &mut ctx)
        .await
        .unwrap();

    // Populate L2 with another key
    let key_l2 = CacheKey::from_str("l2_key", "");
    let value_l2 = CacheValue::new(
        Value {
            name: "in_l2".to_owned(),
            index: 22,
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );
    l2_mem
        .set::<Value>(&key_l2, &value_l2, &mut ctx)
        .await
        .unwrap();

    // Create composition with cloneable backends
    let composition = CompositionBackend::new(l1_mem, l2_mem, TestOffloadManager);

    // Use composition itself as a trait object
    let backend: Box<dyn Backend> = Box::new(composition);

    // Test 1: Read key from L1 through trait object
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = backend.get::<Value>(&key_l1, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().name, "in_l1");

    // Test 2: Read key from L2 through trait object
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = backend.get::<Value>(&key_l2, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().name, "in_l2");

    // Test 3: Write new key through trait object - should go to both layers
    let key_new = CacheKey::from_str("new_key", "");
    let value_new = CacheValue::new(
        Value {
            name: "nested_trait".to_owned(),
            index: 99,
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<Value>(&key_new, &value_new, &mut ctx)
        .await
        .unwrap();

    // Read back the new key
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = backend.get::<Value>(&key_new, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().name, "nested_trait");

    // Test 4: Missing key should return None
    let key_missing = CacheKey::from_str("not_there", "");
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = backend.get::<Value>(&key_missing, &mut ctx).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_composition_with_cache_wrapper() {
    // Test CompositionBackend with the Cache wrapper struct using cloneable backends
    let l1 = MemBackend::new();
    let l2 = MemBackend::new();
    let composition = CompositionBackend::new(l1, l2, TestOffloadManager);

    let cache = Cache::new(composition);

    // The Cache::test() method writes and reads a value
    cache.test().await;
}
