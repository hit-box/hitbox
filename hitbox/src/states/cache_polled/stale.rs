use std::fmt::Debug;

use tracing::{instrument, trace};

use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;
use crate::states::upstream_polled::{
    UpstreamPolledErrorStaleRetrieved, UpstreamPolledStaleRetrieved, UpstreamPolledSuccessful,
};
use crate::CachedValue;
use std::fmt;

pub struct CachePolledStale<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    pub adapter: A,
    pub result: CachedValue<T>,
}

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
                UpstreamPolledStaleRetrieved::Error(UpstreamPolledErrorStaleRetrieved {
                    error,
                    result: self.result.into_inner(),
                })
            }
        }
    }

    #[instrument]
    pub fn finish(self) -> Finish<T> {
        trace!("Finish");
        Finish {
            result: Ok(self.result.into_inner()),
        }
    }
}
