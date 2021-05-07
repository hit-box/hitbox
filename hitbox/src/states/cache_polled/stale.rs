use crate::runtime::RuntimeAdapter;
use crate::CachedValue;
use crate::states::finish::Finish;
use crate::states::upstream_polled::{
    UpstreamPolledErrorStaleRetrieved, UpstreamPolledStaleRetrieved, UpstreamPolledSuccessful,
};
use std::fmt::Debug;
use crate::response::CacheableResponse;

pub struct CachePolledStale<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse
{
    pub adapter: A,
    pub result: CachedValue<T>,
}

impl<A, T> CachePolledStale<A, T>
where
    A: RuntimeAdapter,
    T: Debug + CacheableResponse,
{
    pub async fn poll_upstream(self) -> UpstreamPolledStaleRetrieved<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>,
    {
        match self.adapter.poll_upstream().await {
            Ok(result) => UpstreamPolledStaleRetrieved::Successful(UpstreamPolledSuccessful {
                adapter: self.adapter,
                result,
            }),
            Err(error) => UpstreamPolledStaleRetrieved::Error(UpstreamPolledErrorStaleRetrieved {
                error,
                result: self.result.into_inner(),
            }),
        }
    }
    pub fn finish(self) -> Finish<T> {
        Finish {
            result: Ok(self.result.into_inner()),
        }
    }
}
