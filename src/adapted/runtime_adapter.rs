use std::pin::Pin;
use std::future::Future;
use crate::CacheError;

pub type AdapterResult<T> = Pin<Box<dyn Future<Output=Result<T, CacheError>>>>;

pub trait RuntimeAdapter
{
    type UpstreamResult;
    fn poll_upstream(&self) -> AdapterResult<Self::UpstreamResult>;
}
