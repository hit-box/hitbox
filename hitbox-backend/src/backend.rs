use std::{future::Future, pin::Pin, sync::Arc};

use crate::{response2::CacheableResponse, BackendError, CachedValue, DeleteStatus};
use async_trait::async_trait;

pub type BackendResult<T> = Result<T, BackendError>;
pub type BackendFuture<T> = Pin<Box<dyn Future<Output = BackendResult<T>> + Send>>;

#[async_trait]
pub trait CacheBackend {
    async fn get<T>(&self, key: String) -> BackendResult<Option<CachedValue<T::Cached>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned;

    async fn set<T>(
        &self,
        key: String,
        value: CachedValue<T::Cached>,
        ttl: Option<u32>,
    ) -> BackendResult<()>
    where
        T: CacheableResponse + Send,
        <T as CacheableResponse>::Cached: serde::Serialize + Send;

    async fn delete(&self, key: String) -> BackendResult<DeleteStatus>;

    async fn start(&self) -> BackendResult<()>;
}
