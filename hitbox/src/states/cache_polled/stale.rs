use tracing::{instrument, trace, warn};

#[cfg(feature = "cache-metrics")]
use crate::metrics::{CACHE_HIT_COUNTER, CACHE_STALE_COUNTER};
use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;
use crate::states::upstream_polled::{
    UpstreamPolledErrorStaleRetrieved, UpstreamPolledStaleRetrieved, UpstreamPolledSuccessful,
};
use crate::CacheableResponse;
use crate::CachedValue;
use std::fmt;

/// This state means that the data in the cache is stale.
pub struct CachePolledStale<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    /// Runtime adapter.
    pub adapter: A,
    /// Value retrieved from cache.
    pub result: CachedValue<T>,
}

/// Required `Debug` implementation to use `instrument` macro.
impl<A, T> fmt::Debug for CachePolledStale<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CachePolledStale")
    }
}

impl<A, T> CachePolledStale<A, T>
where
    A: RuntimeAdapter,
    T: fmt::Debug + CacheableResponse,
{
    #[instrument]
    /// Poll data from upstream.
    pub async fn poll_upstream(mut self) -> UpstreamPolledStaleRetrieved<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>,
    {
        let upstream_response = self.adapter.poll_upstream().await;
        #[cfg(feature = "cache-metrics")]
        metrics::increment_counter!(
            CACHE_HIT_COUNTER.as_ref(),
            "upstream" => self.adapter.upstream_name(),
            "message" => self.adapter.message_name(),
        );
        #[cfg(feature = "cache-metrics")]
        metrics::increment_counter!(
            CACHE_STALE_COUNTER.as_ref(),
            "upstream" => self.adapter.upstream_name(),
            "message" => self.adapter.message_name(),
        );
        match upstream_response {
            Ok(result) => {
                trace!("UpstreamPolledSuccessful");
                UpstreamPolledStaleRetrieved::Successful(UpstreamPolledSuccessful {
                    adapter: self.adapter,
                    result,
                })
            }
            Err(error) => {
                trace!("UpstreamPolledErrorStaleRetrieved");
                warn!("Upstream error {}", error);
                UpstreamPolledStaleRetrieved::Error(UpstreamPolledErrorStaleRetrieved {
                    error,
                    result: self.result.into_inner(),
                })
            }
        }
    }

    #[instrument]
    /// Return data with Finish state.
    pub fn finish(self) -> Finish<T> {
        trace!("Finish");
        Finish {
            result: Ok(self.result.into_inner()),
        }
    }
}
