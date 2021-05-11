use std::fmt;

use tracing::{instrument, trace};

use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};

pub struct CacheErrorOccurred<A>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
}

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
            },
            Err(error) => {
                trace!("UpstreamPolledError");
                UpstreamPolled::Error(UpstreamPolledError { error })
            },
        }
    }
}
