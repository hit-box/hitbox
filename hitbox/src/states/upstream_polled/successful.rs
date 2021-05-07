use crate::runtime::RuntimeAdapter;
use crate::states::cache_updated::CacheUpdated;
use crate::states::finish::Finish;
use std::fmt::Debug;
use crate::response::{CacheableResponse, CachePolicy};

pub struct UpstreamPolledSuccessful<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse
{
    pub adapter: A,
    pub result: T,
}

impl<A, T> UpstreamPolledSuccessful<A, T>
where
    A: RuntimeAdapter,
    T: Debug + CacheableResponse
{
    pub fn finish(self) -> Finish<T> {
        Finish {
            result: Ok(self.result),
        }
    }

    pub async fn update_cache(self) -> CacheUpdated<A, T> {
        let result = match self.result.cache_policy() {
            CachePolicy::Cacheable(value) => serde_json::to_vec(value),
            CachePolicy::NonCacheable(value) => Ok(Vec::new()),
        };
        CacheUpdated {
            adapter: self.adapter,
            result: self.result,
        }
    }
}
