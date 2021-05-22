use std::fmt;

use tracing::{instrument, trace, warn};

use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};

/// This state is a variant without data from [CachePolled](enum.CachePolled.html).
pub struct CacheErrorOccurred<A>
where
    A: RuntimeAdapter,
{
    /// Runtime adapter.
    pub adapter: A,
}

/// Required `Debug` implementation to use `instrument` macro.
impl<A> fmt::Debug for CacheErrorOccurred<A>
where
    A: RuntimeAdapter,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CacheErrorOccurred")
    }
}

impl<A> CacheErrorOccurred<A>
where
    A: RuntimeAdapter,
{
    #[instrument]
    /// If we can't retrieve data from cache we have to poll upstream.
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
}
