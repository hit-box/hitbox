
use crate::{CacheState, CacheError, CachedValue};
use std::future::Future;
use std::pin::Pin;
use serde::Serialize;

pub type AdapterResult<T> = Pin<Box<dyn Future<Output = Result<T, CacheError>>>>;

pub trait RuntimeAdapter {
    type UpstreamResult;
    fn poll_upstream(&self) -> AdapterResult<Self::UpstreamResult>;
    fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>>;
    fn update_cache<U: Serialize>(&self, cached_value: CachedValue<U>) -> AdapterResult<()>;
}
