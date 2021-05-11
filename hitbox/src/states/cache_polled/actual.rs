use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;
use crate::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};
use crate::CachedValue;
use std::fmt::Debug;

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
            Ok(result) => UpstreamPolled::Successful(UpstreamPolledSuccessful {
                adapter: self.adapter,
                result,
            }),
            Err(error) => UpstreamPolled::Error(UpstreamPolledError { error }),
        }
    }
    pub fn finish(self) -> Finish<T> {
        Finish {
            result: Ok(self.result.into_inner()),
        }
    }
}
