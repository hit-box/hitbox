use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::states::finish::Finish;
use std::fmt::Debug;
use crate::CacheError;
use crate::states::upstream_polled::{UpstreamPolled, UpstreamPolledSuccessful, UpstreamPolledError};
use crate::adapted::actix_runtime_adapter::{CacheState, CachedValue};
use crate::states::stale_upstream_polled::{StaleUpstreamPolled, StaleUpstreamPolledError};

/// Cache miss state.
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

/// Cache error state.
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

/// Cache has actual data.
pub struct CachePolledActual<A, T>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
    pub result: CachedValue<T>
}

impl<A, T> CachePolledActual<A, T>
where
    A: RuntimeAdapter,
    T: Debug,
{
    pub async fn poll_upstream(self) -> UpstreamPolled<A, T>
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
    pub fn finish(self) -> Finish<T> {
        Finish { result: Ok(self.result.into_inner()) }
    }
}

/// Cache has stale data.
pub struct CachePolledStale<A, T>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
    pub result: CachedValue<T>
}

impl<A, T> CachePolledStale<A, T>
where
    A: RuntimeAdapter,
    T: Debug,
{
    pub async fn poll_upstream(self) -> StaleUpstreamPolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>
    {
        match self.adapter.poll_upstream().await {
            Ok(result) => StaleUpstreamPolled::Successful(
                UpstreamPolledSuccessful { adapter: self.adapter, result }
            ),
            Err(error) => StaleUpstreamPolled::Error(
                StaleUpstreamPolledError { error, result: self.result.into_inner() }
            ),
        }
    }
    pub fn finish(self) -> Finish<T> {
        Finish { result: Ok(self.result.into_inner()) }
    }
}

/// All states.
pub enum CachePolled<A, T>
where
    A: RuntimeAdapter,
{
    Actual(CachePolledActual<A, T>),
    Stale(CachePolledStale<A, T>),
    Miss(CacheMissed<A>),
    Error(CacheErrorOccurred<A>),
}
