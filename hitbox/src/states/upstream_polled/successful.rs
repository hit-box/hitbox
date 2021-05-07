use crate::runtime::RuntimeAdapter;
use crate::states::cache_updated::CacheUpdated;
use crate::states::finish::Finish;
use std::fmt::Debug;
use crate::response::{CacheableResponse, CachePolicy};
use crate::states::cache_policy::{CachePolicyChecked, CachePolicyNonCacheable, CachePolicyCacheable};
use crate::CachedValue;

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
    pub fn check_cache_policy(self) -> CachePolicyChecked<A, T> {
        match self.result.cache_policy() {
            CachePolicy::Cacheable(_) => {
                CachePolicyChecked::Cacheable(
                    CachePolicyCacheable { result: self.result, adapter: self.adapter }
                )
            },
            CachePolicy::NonCacheable(_) => CachePolicyChecked::NonCacheable(
                CachePolicyNonCacheable { result: self.result }
            )
        }
    }
    pub async fn update_cache(self) -> CacheUpdated<A, T> {
        CacheUpdated {
            adapter: self.adapter,
            result: self.result,
        }
    }
}
