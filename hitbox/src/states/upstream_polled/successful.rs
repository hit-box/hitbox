use crate::adapted::runtime_adapter::RuntimeAdapter;
use std::fmt::Debug;
use crate::states::cache_updated::CacheUpdated;
use crate::states::finish::Finish;

pub struct UpstreamPolledSuccessful<A, T>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
    pub result: T
}

impl<A, T> UpstreamPolledSuccessful<A, T>
where
    A: RuntimeAdapter,
    T: Debug,
{
    pub fn finish(self) -> Finish<T> {
        Finish { result: Ok(self.result) }
    }

    pub async fn update_cache(self) -> CacheUpdated<A, T> {
        CacheUpdated { adapter: self.adapter, result: self.result }
    }
}