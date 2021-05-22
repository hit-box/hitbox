use tracing::{instrument, trace, warn};

use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};
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
