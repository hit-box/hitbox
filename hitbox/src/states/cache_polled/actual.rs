use std::fmt::Debug;

use tracing::trace;

use crate::CachedValue;
use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;
use crate::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};

pub struct CachePolledActual<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    pub adapter: A,
    pub result: CachedValue<T>,
}

impl<A, T> CachePolledActual<A, T>
where
    A: RuntimeAdapter,
    T: Debug + CacheableResponse,
{
    pub async fn poll_upstream(mut self) -> UpstreamPolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>,
    {
        match self.adapter.poll_upstream().await {
            Ok(result) => {
                trace!("-> UpstreamPolledSuccessful");
                UpstreamPolled::Successful(UpstreamPolledSuccessful {
                    adapter: self.adapter,
                    result,
                })
            },
            Err(error) => {
                trace!("-> UpstreamPolledError");
                UpstreamPolled::Error(UpstreamPolledError { error })
            },
        }
    }
    pub fn finish(self) -> Finish<T> {
        trace!("-> Finish");
        Finish {
            result: Ok(self.result.into_inner()),
        }
    }
}
