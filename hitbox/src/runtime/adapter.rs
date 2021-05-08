use crate::response::CacheableResponse;
use crate::{CacheError, CacheState, CachedValue};
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;

pub type AdapterResult<T> = Pin<Box<dyn Future<Output = Result<T, CacheError>>>>;

pub trait RuntimeAdapter
where
    Self::UpstreamResult: CacheableResponse,
{
    type UpstreamResult;
    fn poll_upstream(&self) -> AdapterResult<Self::UpstreamResult>;
    fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>>;
    fn update_cache(&self, cached_value: &CachedValue<Self::UpstreamResult>) -> AdapterResult<()>;
}
