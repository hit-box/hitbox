use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::settings::InitialCacheSettings;
use crate::states::cache_polled::{
    CacheErrorOccurred, CacheMissed, CachePolled, CachePolledActual, CachePolledStale,
};
use crate::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};
use crate::CacheError;
use crate::CacheState;
use std::fmt::Debug;

#[derive(Debug)]
pub struct InitialState<A>
where
    A: RuntimeAdapter,
{
    pub settings: InitialCacheSettings,
    pub adapter: A,
}

impl<A> InitialState<A>
where
    A: RuntimeAdapter,
{
    pub async fn poll_upstream<T>(self) -> UpstreamPolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>,
        T: CacheableResponse,
    {
        match self.adapter.poll_upstream().await {
            Ok(result) => UpstreamPolled::Successful(UpstreamPolledSuccessful {
                adapter: self.adapter,
                result,
            }),
            Err(error) => UpstreamPolled::Error(UpstreamPolledError { error }),
        }
    }

    pub async fn poll_cache<T>(self) -> CachePolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>,
        T: CacheableResponse,
    {
        let cache_result: Result<CacheState<T>, CacheError> = self.adapter.poll_cache().await;
        match cache_result {
            Ok(value) => match value {
                CacheState::Actual(result) => CachePolled::Actual(CachePolledActual {
                    adapter: self.adapter,
                    result,
                }),
                CacheState::Stale(result) => CachePolled::Stale(CachePolledStale {
                    adapter: self.adapter,
                    result,
                }),
                CacheState::Miss => CachePolled::Miss(CacheMissed {
                    adapter: self.adapter,
                }),
            },
            Err(_) => CachePolled::Error(CacheErrorOccurred {
                adapter: self.adapter,
            }),
        }
    }
}
