use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use hitbox_backend::{Backend, BackendResult, CacheBackend, CacheKeyFormat, serializer::Raw};
use hitbox_core::{CacheKey, CacheValue, CacheableResponse, EntityPolicyConfig};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug)]
struct MemBackend {
    storage: RwLock<HashMap<String, Vec<u8>>>,
}

impl MemBackend {
    fn new() -> Self {
        let mut storage = HashMap::new();
        // URL-encoded format: key1=
        storage.insert(
            "key1=".to_owned(),
            b"{\"name\": \"test\", \"index\": 42}".to_vec(),
        );
        MemBackend {
            storage: RwLock::new(storage),
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

    async fn write(
        &self,
        key: &CacheKey,
        value: CacheValue<Raw>,
        _ttl: Option<std::time::Duration>,
    ) -> BackendResult<()> {
        let mut lock = self.storage.write().await;
        let key_str = String::from_utf8(CacheKeyFormat::UrlEncoded.serialize(key)?).unwrap();
        lock.insert(key_str, value.data);
        Ok(())
    }

    async fn remove(&self, _key: &CacheKey) -> BackendResult<hitbox_backend::DeleteStatus> {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Value {
    name: String,
    index: u8,
}

#[async_trait]
impl CacheableResponse for Value {
    type Cached = Self;
    type Subject = Self;

    async fn cache_policy<P>(
        self,
        _predicates: P,
        _: &EntityPolicyConfig,
    ) -> Result<hitbox_core::ResponseCachePolicy<Self>, hitbox_core::PredicateError>
    where
        P: hitbox_core::Predicate<Subject = Self::Subject> + Send + Sync,
    {
        todo!()
    }

    async fn into_cached(self) -> hitbox_core::CachePolicy<Self::Cached, Self> {
        todo!()
    }

    async fn from_cached(_cached: Self::Cached) -> Self {
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
        self.backend
            .set::<Value>(&CacheKey::from_str("key3", ""), &value, None)
            .await
            .unwrap();
        dbg!(
            self.backend
                .get::<Value>(&CacheKey::from_str("key3", ""))
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
    let value = storage.get::<Value>(&key1).await.unwrap();
    dbg!(value);

    let backend: Box<dyn Backend> = Box::new(storage);
    let value = backend.get::<Value>(&key1).await.unwrap();
    dbg!(value);

    let value = CacheValue::new(
        Value {
            name: "value2".to_owned(),
            index: 255,
        },
        Some(Utc::now()),
        Some(Utc::now()),
    );
    backend.set::<Value>(&key2, &value, None).await.unwrap();
    let value = backend.get::<Value>(&key2).await.unwrap();
    dbg!(value);

    let cache = Cache::new(backend);
    cache.test().await;

    let cache = Cache::new(MemBackend::new());
    cache.test().await;
}

// #[async_trait]
// impl CacheBackend for DummyBackend {
//     async fn get<T>(
//         &self,
//         key: &hitbox_core::CacheKey,
//     ) -> hitbox_backend::BackendResult<Option<hitbox_core::CachedValue<T::Cached>>>
//     where
//         T: hitbox_core::CacheableResponse,
//         <T as hitbox_core::CacheableResponse>::Cached: serde::de::DeserializeOwned,
//     {
//         todo!()
//     }
//
//     async fn set<T>(
//         &self,
//         key: &hitbox_core::CacheKey,
//         value: &hitbox_core::CachedValue<T::Cached>,
//         ttl: Option<u32>,
//     ) -> hitbox_backend::BackendResult<()>
//     where
//         T: hitbox_core::CacheableResponse + Send,
//         T::Cached: serde::Serialize + Send + Sync,
//     {
//         todo!()
//     }
//
//     async fn delete(
//         &self,
//         key: &hitbox_core::CacheKey,
//     ) -> hitbox_backend::BackendResult<hitbox_backend::DeleteStatus> {
//         todo!()
//     }
//
//     async fn start(&self) -> hitbox_backend::BackendResult<()> {
//         todo!()
//     }
// }
//
// #[derive(Clone, Debug)]
// struct A {}
//
// #[async_trait]
// impl CacheableResponse for A {
//     type Cached = u8;
//
//     type Subject = A;
//
//     async fn cache_policy<P>(self, predicates: P) -> hitbox_core::ResponseCachePolicy<Self>
//     where
//         P: hitbox_core::Predicate<Subject = Self::Subject> + Send + Sync,
//     {
//         todo!()
//     }
//
//     async fn into_cached(self) -> hitbox_core::CachePolicy<Self::Cached, Self> {
//         todo!()
//     }
//
//     async fn from_cached(cached: Self::Cached) -> Self {
//         todo!()
//     }
// }
//
// #[tokio::test]
// async fn test_dyn_backend() {
//     let backend: Box<dyn ErasedBackend> = Box::new(DummyBackend {});
//     let key = CacheKey::from_str("test", "key");
//     let value = CachedValue::new(42, Utc::now());
//     let result = backend.set::<A>(&key, &value, None).await;
//     dbg!(result);
//
//     let result = backend.get::<A>(&key).await;
//     dbg!(result);
// }
