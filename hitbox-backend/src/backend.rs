// use std::pin::Pin;
// use std::future::Future;
use async_trait::async_trait;
use crate::{CachedValue, BackendError, DeleteStatus};

// type BackendResult<T> = Pin<Box<dyn Future<Output=Result<T, BackendError>>>>;
pub type BackendResult<T> = Result<T, BackendError>;


#[async_trait]
pub trait CacheBackend {
    async fn get<T>(&self, key: String) -> BackendResult<CachedValue<T>>;
    async fn set<T: Send>(&self, key: String, value: CachedValue<T>, ttl: Option<u32>) -> BackendResult<()>;
    async fn delete(&self, key: String) -> BackendResult<DeleteStatus>;
}
