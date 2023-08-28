use crate::{BackendError, DeleteStatus};
use async_trait::async_trait;
use hitbox_core::{CacheKey, CachedValue};

pub type BackendResult<T> = Result<T, BackendError>;

#[async_trait]
pub trait CacheBackend<T>
where
    //T: CacheableResponse + Send,
    T: serde::de::DeserializeOwned + serde::Serialize + Send + Sync,
{
    async fn get(&self, key: &CacheKey) -> BackendResult<Option<CachedValue<T>>>;

    async fn set(
        &self,
        key: &CacheKey,
        value: &CachedValue<T>,
        ttl: Option<u32>,
    ) -> BackendResult<()>;

    async fn delete(&self, key: &CacheKey) -> BackendResult<DeleteStatus>;

    async fn start(&self) -> BackendResult<()>;
}
