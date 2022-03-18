use crate::{BackendError, CacheableResponse, CachedValue, DeleteStatus};
use async_trait::async_trait;

pub type BackendResult<T> = Result<T, BackendError>;

#[async_trait]
pub trait CacheBackend {
    async fn get<T>(&self, key: String) -> BackendResult<Option<CachedValue<T>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned;
    async fn set<T>(
        &self,
        key: String,
        value: &CachedValue<T>,
        ttl: Option<u32>,
    ) -> BackendResult<()>
    where
        T: CacheableResponse + Sync,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned;
    async fn delete(&self, key: String) -> BackendResult<DeleteStatus>;
    async fn start(&self) -> BackendResult<()>;
}
