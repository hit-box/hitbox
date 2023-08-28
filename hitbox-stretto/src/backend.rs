use crate::builder::StrettoBackendBuilder;
use async_trait::async_trait;
use hitbox::{CacheKey, CachedValue};
use hitbox_backend::{
    serializer::{BinSerializer, Serializer},
    BackendError, BackendResult, CacheBackend, DeleteStatus,
};
use std::time::Duration;
use stretto::AsyncCache;

#[derive(Clone)]
pub struct StrettoBackend {
    pub(crate) cache: AsyncCache<CacheKey, Vec<u8>>,
}

impl StrettoBackend {
    pub fn builder(max_size: i64) -> StrettoBackendBuilder {
        StrettoBackendBuilder::new(max_size)
    }
}

#[async_trait]
impl<T> CacheBackend<T> for StrettoBackend
where
    T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync,
{
    async fn get(&self, key: &CacheKey) -> BackendResult<Option<CachedValue<T>>> {
        let () = self
            .cache
            .wait()
            .await
            .map_err(crate::error::Error::from)
            .map_err(BackendError::from)?;

        match self.cache.get(key).await {
            Some(cached) => BinSerializer::<Vec<u8>>::deserialize(cached.value().to_owned())
                .map_err(BackendError::from)
                .map(Some),

            None => Ok(None),
        }
    }

    async fn set(
        &self,
        key: &CacheKey,
        value: &CachedValue<T>,
        ttl: Option<u32>,
    ) -> BackendResult<()> {
        let serialized = BinSerializer::<Vec<u8>>::serialize(value).map_err(BackendError::from)?;
        let cost = serialized.len();
        let inserted = match ttl {
            Some(ttl) => {
                self.cache
                    .insert_with_ttl(
                        key.to_owned(),
                        serialized,
                        cost as i64,
                        Duration::from_secs(ttl as u64),
                    )
                    .await
            }
            None => {
                self.cache
                    .insert(key.to_owned(), serialized, cost as i64)
                    .await
            }
        };
        if inserted {
            Ok(())
        } else {
            Err(BackendError::from(crate::error::Error::Insert))
        }
    }

    async fn delete(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        self.cache.remove(key).await;
        Ok(DeleteStatus::Deleted(1))
    }

    async fn start(&self) -> BackendResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use async_trait::async_trait;
    use chrono::Utc;
    use serde::{Deserialize, Serialize};

    use super::*;
    use hitbox::CacheableResponse;

    #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
    struct Test {
        a: i32,
        b: String,
    }

    #[async_trait]
    impl CacheableResponse for Test {
        type Cached = Self;

        async fn into_cached(self) -> Self::Cached {
            self
        }
        async fn from_cached(cached: Self::Cached) -> Self {
            cached
        }
    }

    impl Test {
        pub fn new() -> Self {
            Self {
                a: 42,
                b: "nope".to_owned(),
            }
        }
    }

    #[tokio::test]
    async fn test_set_and_get() {
        let cache = crate::StrettoBackend::builder(100).finalize().unwrap();
        let value = CachedValue::new(Test::new(), Utc::now());
        let key = CacheKey::from_str("key", "1");
        let res = cache.set::<Test>(&key, &value, None).await;
        assert!(res.is_ok());
        let value = cache.get::<Test>(&key).await.unwrap().unwrap().into_inner();
        assert_eq!(value.a, 42);
        assert_eq!(value.b, "nope".to_owned());
    }
}
