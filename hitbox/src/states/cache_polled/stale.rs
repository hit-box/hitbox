use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::cache::CachedValue;
use std::fmt::Debug;
use crate::states::upstream_polled::{UpstreamPolledSuccessful, UpstreamPolledStaleRetrieved, UpstreamPolledErrorStaleRetrieved};
use crate::states::finish::Finish;

pub struct CachePolledStale<A, T>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
    pub result: CachedValue<T>
}

impl<A, T> CachePolledStale<A, T>
where
    A: RuntimeAdapter,
    T: Debug,
{
    pub async fn poll_upstream(self) -> UpstreamPolledStaleRetrieved<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>
    {
        match self.adapter.poll_upstream().await {
            Ok(result) => UpstreamPolledStaleRetrieved::Successful(
                UpstreamPolledSuccessful { adapter: self.adapter, result }
            ),
            Err(error) => UpstreamPolledStaleRetrieved::Error(
                UpstreamPolledErrorStaleRetrieved { error, result: self.result.into_inner() }
            ),
        }
    }
    pub fn finish(self) -> Finish<T> {
        Finish { result: Ok(self.result.into_inner()) }
    }
}
