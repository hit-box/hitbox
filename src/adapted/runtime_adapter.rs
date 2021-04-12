use std::pin::Pin;
use std::future::Future;
use crate::{CacheError, adapted::actix_runtime_adapter::CacheState};

pub type AdapterResult<T> = Pin<Box<dyn Future<Output=Result<T, CacheError>>>>;

pub trait RuntimeAdapter
{
    type UpstreamResult;
    fn poll_upstream(&self) -> AdapterResult<Self::UpstreamResult>;
    fn poll_cache(&self) -> AdapterResult<crate::adapted::actix_runtime_adapter::CacheState<Self::UpstreamResult>>;
}
