use crate::CachedValue;
use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;
use crate::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};
use std::fmt::Debug;

pub struct CachePolledActual<A, T>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
    pub result: CachedValue<T>,
}

impl<A, T> CachePolledActual<A, T>
where
    A: RuntimeAdapter,
    T: Debug,
{
    pub async fn poll_upstream(self) -> UpstreamPolled<A, T>
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
