use std::fmt::Debug;

use tracing::{instrument, trace, warn};

use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;
use crate::states::upstream_polled::{
    UpstreamPolledErrorStaleRetrieved, UpstreamPolledStaleRetrieved, UpstreamPolledSuccessful,
};
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
    T: Debug + CacheableResponse,
{
    #[instrument]
    /// Poll data from upstream.
    pub async fn poll_upstream(mut self) -> UpstreamPolledStaleRetrieved<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>,
    {
        match self.adapter.poll_upstream().await {
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
