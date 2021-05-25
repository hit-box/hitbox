use std::fmt;

use tracing::{instrument, trace, warn};

use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::settings::{InitialCacheSettings, CacheSettings};
use crate::states::cache_polled::{
    CacheErrorOccurred, CacheMissed, CachePolled, CachePolledActual, CachePolledStale,
};
use crate::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};
use crate::transition_groups::{only_cache, stale, upstream};
use crate::{CacheError, CacheState};

pub struct Initial<A>
where
    A: RuntimeAdapter,
{
    settings: InitialCacheSettings,
    pub adapter: A,
}

impl<A> fmt::Debug for Initial<A>
where
    A: RuntimeAdapter,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Initial")
    }
}

impl<A> Initial<A>
where
    A: RuntimeAdapter,
{
    pub fn new(settings: CacheSettings, adapter: A) -> Self {
        Self {
            settings: InitialCacheSettings::from(settings),
            adapter
        }
    }

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
                warn!("Upstream error {}", error);
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
            Err(error) => {
                trace!("CacheErrorOccurred");
                warn!("Cache error {}", error);
                CachePolled::Error(CacheErrorOccurred {
                    adapter: self.adapter,
                })
            }
        }
    }

    pub async fn transitions<T>(self) -> Result<T, CacheError>
    where
        A: RuntimeAdapter<UpstreamResult = T>,
        T: CacheableResponse + fmt::Debug,
    {
        match self.settings {
            InitialCacheSettings::CacheDisabled => {
                upstream::transition(self).await.result()
            }
            InitialCacheSettings::CacheEnabled => {
                only_cache::transition(self).await.result()
            }
            InitialCacheSettings::CacheStale => stale::transition(self).await.result(),
            InitialCacheSettings::CacheLock => unimplemented!(),
            InitialCacheSettings::CacheStaleLock => unimplemented!(),
        }
    }
}
