use std::{pin::Pin, future::Future, sync::Arc};

use crate::{BackendError, CacheableResponse, CachedValue, DeleteStatus};
use async_trait::async_trait;

pub type BackendResult<T> = Result<T, BackendError>;
pub type BackendFuture<T> = Pin<Box<dyn Future<Output = BackendResult<T>> + Send>>;

#[async_trait]
pub trait CacheBackend 
where
    Self: 'static,
{
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

pub trait Backend {
    fn get<'a, T>(&'a self, key: String) -> BackendFuture<Option<CachedValue<T>>>
    where
        T: CacheableResponse + 'a,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned;
}

#[async_trait]
impl<B> CacheBackend for Arc<B> 
where
    B: CacheBackend + Send + Sync,
{
    async fn get<T>(&self, key: String) -> BackendResult<Option<CachedValue<T>>>
    where
        T: CacheableResponse,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned {
            self.get(key).await
        }
    async fn set<T>(
        &self,
        key: String,
        value: &CachedValue<T>,
        ttl: Option<u32>,
    ) -> BackendResult<()>
    where
        T: CacheableResponse + Sync,
        <T as CacheableResponse>::Cached: serde::de::DeserializeOwned {
            self.set(key, value, ttl).await
        }
    async fn delete(&self, key: String) -> BackendResult<DeleteStatus> {
        self.delete(key).await
    }
    async fn start(&self) -> BackendResult<()> {
        self.start().await
    }
}
