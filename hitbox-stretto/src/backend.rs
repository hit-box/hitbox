use crate::builder::StrettoBackendBuilder;
use axum::async_trait;
use hitbox_backend::{
    serializer::{BinSerializer, Serializer},
    BackendError, BackendResult, CacheBackend, CacheableResponse, CachedValue, DeleteStatus,
};
use std::time::Duration;
use stretto::AsyncCache;

#[derive(Clone)]
pub struct StrettoBackend {
    pub(crate) cache: AsyncCache<String, Vec<u8>>,
}

impl StrettoBackend {
    pub fn builder(max_size: i64) -> StrettoBackendBuilder {
        StrettoBackendBuilder::new(max_size)
    }
}

#[async_trait]
impl CacheBackend for StrettoBackend {
    async fn get<T>(&self, key: String) -> BackendResult<Option<CachedValue<T::Cached>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned,
    {
        let () = self
            .cache
            .wait()
            .await
            .map_err(crate::error::Error::from)
            .map_err(BackendError::from)?;

        match self.cache.get(&key).await {
            Some(cached) => BinSerializer::<Vec<u8>>::deserialize(cached.value().to_owned())
                .map_err(BackendError::from)
                .map(Some),

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
        let serialized = BinSerializer::<Vec<u8>>::serialize(&value).map_err(BackendError::from)?;
        let cost = serialized.len();
        let inserted = match ttl {
            Some(ttl) => {
                self.cache
                    .insert_with_ttl(
                        key,
                        serialized,
                        cost as i64,
                        Duration::from_secs(ttl as u64),
                    )
                    .await
            }
            None => self.cache.insert(key, serialized, cost as i64).await,
        };
        if inserted {
            Ok(())
        } else {
            Err(BackendError::from(crate::error::Error::Insert))
        }
    }

    async fn delete(&self, key: String) -> BackendResult<DeleteStatus> {
        self.cache.remove(&key).await;
        Ok(DeleteStatus::Deleted(1))
    }

    async fn start(&self) -> BackendResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use axum::async_trait;
    use chrono::Utc;
    use serde::{Deserialize, Serialize};

    use super::*;
    use hitbox_backend::CacheableResponse;

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
        let res = cache.set::<Test>("key-1".to_string(), &value, None).await;
        assert!(res.is_ok());
        let value = cache
            .get::<Test>("key-1".to_string())
            .await
            .unwrap()
            .unwrap()
            .into_inner();
        assert_eq!(value.a, 42);
        assert_eq!(value.b, "nope".to_owned());
    }
}