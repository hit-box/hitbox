use async_trait::async_trait;
use chrono::Utc;
use hitbox::{CacheKey, CacheValue};
use hitbox_backend::Backend;
use hitbox_backend::serializer::{Format, JsonFormat};
use hitbox_backend::{
    BackendResult, CacheKeyFormat, Compressor, DeleteStatus, PassthroughCompressor,
};
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

#[derive(Clone)]
pub struct MokaBackend<S = JsonFormat, C = PassthroughCompressor>
where
    S: Format,
    C: Compressor,
{
    pub cache: Cache<CacheKey, CacheValue<Raw>>,
    pub key_format: CacheKeyFormat,
    pub serializer: S,
    pub compressor: C,
}

impl<S, C> std::fmt::Debug for MokaBackend<S, C>
where
    S: Format,
    C: Compressor,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MokaBackend")
            .field("cache", &self.cache)
            .field("key_format", &self.key_format)
            .field("serializer", &std::any::type_name::<S>())
            .field("compressor", &std::any::type_name::<C>())
            .finish()
    }
}

impl MokaBackend<JsonFormat, PassthroughCompressor> {
    pub fn builder(
        max_capacity: u64,
    ) -> crate::builder::MokaBackendBuilder<JsonFormat, PassthroughCompressor> {
        crate::builder::MokaBackendBuilder::new(max_capacity)
    }
}

#[async_trait]
impl<S, C> Backend for MokaBackend<S, C>
where
    S: Format + Send + Sync,
    C: Compressor + Send + Sync,
{
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

    fn value_format(&self) -> &dyn Format {
        &self.serializer
    }

    fn key_format(&self) -> &CacheKeyFormat {
        &self.key_format
    }

    fn compressor(&self) -> &dyn Compressor {
        &self.compressor
    }
}
