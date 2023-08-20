use crate::{BackendError, DeleteStatus};
use async_trait::async_trait;
use hitbox_core::{CacheKey, CacheableResponse, CachedValue};

pub type BackendResult<T> = Result<T, BackendError>;

#[async_trait]
pub trait CacheBackend {
    async fn get<T>(&self, key: &CacheKey) -> BackendResult<Option<CachedValue<T::Cached>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned;

    async fn set<T>(
        &self,
        key: &CacheKey,
        value: &CachedValue<T::Cached>,
        ttl: Option<u32>,
    ) -> BackendResult<()>
    where
        T: CacheableResponse + Send,
        T::Cached: serde::Serialize + Send + Sync;

    async fn delete(&self, key: &CacheKey) -> BackendResult<DeleteStatus>;

    async fn start(&self) -> BackendResult<()>;
}
