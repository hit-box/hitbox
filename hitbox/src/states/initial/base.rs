use std::fmt;
use std::fmt::Debug;

use tracing::{instrument, trace};

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

pub struct InitialState<A>
where
    A: RuntimeAdapter,
{
    pub settings: InitialCacheSettings,
    pub adapter: A,
}

impl<A> fmt::Debug for InitialState<A>
where
    A: RuntimeAdapter,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("InitialState")
    }
}

impl<A> InitialState<A>
where
    A: RuntimeAdapter,
{
    #[instrument]
    pub async fn poll_upstream<T>(mut self) -> UpstreamPolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>,
        T: CacheableResponse,
    {
        match self.adapter.poll_upstream().await {
            Ok(result) => {
                trace!("UpstreamPolledSuccessful");
                UpstreamPolled::Successful(UpstreamPolledSuccessful {
                    adapter: self.adapter,
                    result,
                })
            }
            Err(error) => {
                trace!("UpstreamPolledError");
                UpstreamPolled::Error(UpstreamPolledError { error })
            }
        }
    }

    #[instrument]
    pub async fn poll_cache<T>(self) -> CachePolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>,
        T: CacheableResponse,
    {
        let cache_result: Result<CacheState<T>, CacheError> = self.adapter.poll_cache().await;
        match cache_result {
            Ok(value) => match value {
                CacheState::Actual(result) => {
                    trace!("CachePolledActual");
                    CachePolled::Actual(CachePolledActual {
                        adapter: self.adapter,
                        result,
                    })
                }
                CacheState::Stale(result) => {
                    trace!("CachePolledStale");
                    CachePolled::Stale(CachePolledStale {
                        adapter: self.adapter,
                        result,
                    })
                }
                CacheState::Miss => {
                    trace!("CacheMissed");
                    CachePolled::Miss(CacheMissed {
                        adapter: self.adapter,
                    })
                }
            },
            Err(_) => {
                trace!("CacheErrorOccurred");
                CachePolled::Error(CacheErrorOccurred {
                    adapter: self.adapter,
                })
            }
        }
    }
}
