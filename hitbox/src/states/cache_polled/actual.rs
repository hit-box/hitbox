use std::fmt::Debug;

use tracing::{instrument, trace};

use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;
use crate::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};
use crate::CachedValue;
use std::fmt;

pub struct CachePolledActual<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    pub adapter: A,
    pub result: CachedValue<T>,
}

impl<A, T> fmt::Debug for CachePolledActual<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CachePolledActual")
    }
}

impl<A, T> CachePolledActual<A, T>
where
    A: RuntimeAdapter,
    T: Debug + CacheableResponse,
{
    #[instrument]
    pub async fn poll_upstream(mut self) -> UpstreamPolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>,
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
    pub fn finish(self) -> Finish<T> {
        trace!("Finish");
        Finish {
            result: Ok(self.result.into_inner()),
        }
    }
}
