
use crate::{CacheState, CacheError};
use std::future::Future;
use std::pin::Pin;

pub type AdapterResult<T> = Pin<Box<dyn Future<Output = Result<T, CacheError>>>>;

pub trait RuntimeAdapter {
    type UpstreamResult;
    fn poll_upstream(&self) -> AdapterResult<Self::UpstreamResult>;
    fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>>;
}
