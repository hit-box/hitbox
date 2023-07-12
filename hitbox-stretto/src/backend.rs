use axum::async_trait;
use hitbox_backend::{
    serializer::{JsonSerializer, Serializer},
    BackendError, BackendResult, CacheBackend, CacheableResponse, CachedValue, DeleteStatus,
};
use std::time::Duration;
use stretto::AsyncCache;

const COST: i64 = 0;

#[derive(Clone)]
pub struct StrettoBackend {
    cache: AsyncCache<String, Vec<u8>>,
}

impl StrettoBackend {
    pub fn new(cache: AsyncCache<String, Vec<u8>>) -> Self {
        Self { cache }
    }
}

#[async_trait]
impl CacheBackend for StrettoBackend {
    async fn get<T>(&self, key: String) -> BackendResult<Option<CachedValue<T::Cached>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned,
    {
        match self.cache.get(&key).await {
            Some(cached) => Ok(Some(
                JsonSerializer::<Vec<u8>>::deserialize(cached.value().to_owned())
                    .map_err(BackendError::from)
                    .unwrap(),
            )),
            None => Ok(None),
        }
    }

    async fn set<T>(
        &self,
        key: String,
        value: &CachedValue<T::Cached>,
        ttl: Option<u32>,
    ) -> BackendResult<()>
    where
        T: CacheableResponse + Send,
        T::Cached: serde::Serialize + Send + Sync,
    {
        let serialized =
            JsonSerializer::<Vec<u8>>::serialize(&value).map_err(BackendError::from)?;
        let inserted = match ttl {
            Some(ttl) => {
                self.cache
                    .insert_with_ttl(key, serialized, COST, Duration::from_secs(ttl as u64))
                    .await
            }
            None => self.cache.insert(key, serialized, COST).await,
        };
        if inserted {
            Ok(())
        } else {
            Err(BackendError::from(crate::error::Error::Insert))
        }
    }

    async fn delete(&self, key: String) -> BackendResult<DeleteStatus> {
        self.cache.remove(&key).await;
        Ok(DeleteStatus::Deleted(0))
    }

    async fn start(&self) -> BackendResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test() {
    let c: AsyncCache<String, String> = AsyncCache::new(1000, 100, tokio::spawn).unwrap();

    for i in 0..100 {
        let key = format!("key-{}", i);
        let r = c.insert(key, "value".to_string(), 1).await;
        dbg!(r);
    }

    c.wait().await.unwrap();

    for i in 0..100 {
        let key = format!("key-{}", i);
        let value = c.get(&key).await;
        match value {
            Some(v) => dbg!(v.to_string()),
            None => dbg!("None".to_string()),
        };
    }
}
