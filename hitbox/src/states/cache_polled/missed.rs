use tracing::{trace, instrument};

use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};
use std::fmt;

pub struct CacheMissed<A>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
}

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
