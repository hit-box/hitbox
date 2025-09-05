use async_trait::async_trait;
use chrono::Utc;
use hitbox::{CacheKey, CacheValue};
use hitbox_backend::Backend;
use hitbox_backend::{BackendResult, DeleteStatus};
use moka::{Expiry, future::Cache};
use std::time::{Duration, Instant};

type Raw = Vec<u8>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Expiration;

impl Expiry<CacheKey, CacheValue<Raw>> for Expiration {
    fn expire_after_create(
        &self,
        _key: &CacheKey,
        value: &CacheValue<Raw>,
        _created_at: Instant,
    ) -> Option<Duration> {
        value.expire.map(|expiration| {
            let delta = expiration - Utc::now();
            Duration::from_secs(delta.num_seconds() as u64)
        })
    }
}

#[derive(Clone, Debug)]
pub struct MokaBackend {
    pub cache: Cache<CacheKey, CacheValue<Raw>>,
}

impl MokaBackend {
    pub fn builder(max_capacity: u64) -> crate::builder::MokaBackendBuilder {
        crate::builder::MokaBackendBuilder::new(max_capacity)
    }
}

#[async_trait]
impl Backend for MokaBackend {
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
        self.cache.get(key).await.map(Ok).transpose()
    }

    async fn write(
        &self,
        key: &CacheKey,
        value: CacheValue<Raw>,
        _ttl: Option<Duration>,
    ) -> BackendResult<()> {
        self.cache.insert(key.clone(), value).await;
        Ok(())
    }

    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        let value = self.cache.remove(key).await;
        // FIXME: No need to have u32 inside Deleted option. We can remove it
        match value {
            Some(_) => Ok(DeleteStatus::Deleted(1)),
            None => Ok(DeleteStatus::Missing),
        }
    }
}
