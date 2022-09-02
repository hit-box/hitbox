use tracing::{instrument, trace, warn};

use crate::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};
#[cfg(feature = "metrics")]
use crate::metrics::{CACHE_MISS_COUNTER, CACHE_UPSTREAM_HANDLING_HISTOGRAM};
use std::fmt;

/// This state means that there is no cached data.
pub struct CacheMissed<A>
where
    A: RuntimeAdapter,
{
    /// Runtime adapter.
    pub adapter: A,
}

/// Required `Debug` implementation to use `instrument` macro.
impl<A> fmt::Debug for CacheMissed<A>
where
    A: RuntimeAdapter,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CacheMissed")
    }
}

impl<A> CacheMissed<A>
where
    A: RuntimeAdapter,
{
    #[instrument]
    /// Poll data from upstream.
    pub async fn poll_upstream<T>(mut self) -> UpstreamPolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>,
        T: CacheableResponse,
    {
        #[cfg(feature = "metrics")]
        let timer = std::time::Instant::now();
        let upstream_response = self.adapter.poll_upstream().await;
        #[cfg(feature = "metrics")]
        metrics::histogram!(
            CACHE_UPSTREAM_HANDLING_HISTOGRAM.as_ref(),
            timer.elapsed().as_millis() as f64 / 1000.0,
            "upstream" => self.adapter.upstream_name(),
            "message" => self.adapter.message_name(),
        );
        #[cfg(feature = "metrics")]
        metrics::increment_counter!(
            CACHE_MISS_COUNTER.as_ref(),
            "upstream" => self.adapter.upstream_name(),
            "message" => self.adapter.message_name(),
        );
        match upstream_response {
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
}
