use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::states::finish::Finish;
use std::fmt::Debug;
use crate::CacheError;
use crate::states::upstream_polled::{UpstreamPolled, UpstreamPolledSuccessful, UpstreamPolledError};
use crate::adapted::actix_runtime_adapter::{CacheState, CachedValue};

pub struct CacheMissed<A>
    where
        A: RuntimeAdapter,
{
    pub adapter: A,
}

impl<A> CacheMissed<A>
where
    A: RuntimeAdapter,
{
    pub async fn poll_upstream<T>(self) -> UpstreamPolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>
    {
        match self.adapter.poll_upstream().await {
            Ok(result) => UpstreamPolled::Successful(
                UpstreamPolledSuccessful { adapter: self.adapter, result }
            ),
            Err(error) => UpstreamPolled::Error(UpstreamPolledError { error }),
        }
    }
}

pub struct CacheErrorOccurred<A>
    where
        A: RuntimeAdapter,
{
    pub adapter: A,
}

impl<A> CacheErrorOccurred<A>
where
    A: RuntimeAdapter,
{
    pub async fn poll_upstream<T>(self) -> UpstreamPolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>
    {
        match self.adapter.poll_upstream().await {
            Ok(result) => UpstreamPolled::Successful(
                UpstreamPolledSuccessful { adapter: self.adapter, result }
            ),
            Err(error) => UpstreamPolled::Error(UpstreamPolledError { error }),
        }
    }
}

pub struct CachePolledSuccessful<A, T>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
    pub result: CachedValue<T>
}

impl<A, T> CachePolledSuccessful<A, T>
where
    A: RuntimeAdapter,
    T: Debug,
{
    pub fn finish(self) -> Finish<T> {
        Finish { result: self.result.into_inner() }
    }
}

pub enum CachePolled<A, T>
where
    A: RuntimeAdapter,
{
    Actual(CachePolledSuccessful<A, T>),
    Stale(CachePolledSuccessful<A, T>),
    Miss(CacheMissed<A>),
    Error(CacheErrorOccurred<A>),
}
