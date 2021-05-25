use std::fmt;
use std::fmt::Debug;

use tracing::{instrument, trace, warn};

use crate::CachedValue;
use crate::response::{CacheableResponse, CachePolicy};
use crate::runtime::RuntimeAdapter;
use crate::states::cache_policy::{
    CachePolicyCacheable, CachePolicyChecked, CachePolicyNonCacheable,
};
use crate::states::cache_updated::CacheUpdated;
use crate::states::finish::Finish;

/// Upstream returns value.
pub struct UpstreamPolledSuccessful<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    /// Runtime adapter.
    pub adapter: A,
    /// Value from upstream.
    pub result: T,
}

/// Required `Debug` implementation to use `instrument` macro.
impl<A, T> fmt::Debug for UpstreamPolledSuccessful<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UpstreamPolledSuccessful")
    }
}

impl<A, T> UpstreamPolledSuccessful<A, T>
where
    A: RuntimeAdapter<UpstreamResult = T>,
    T: Debug + CacheableResponse,
{
    #[instrument]
    /// Return retrieved value.
    pub fn finish(self) -> Finish<T> {
        trace!("Finish");
        Finish {
            result: Ok(self.result),
        }
    }

    #[instrument]
    /// Check if the value can be cached.
    pub fn check_cache_policy(self) -> CachePolicyChecked<A, T> {
        match self.result.cache_policy() {
            CachePolicy::Cacheable(_) => {
                trace!("CachePolicyCacheable");
                CachePolicyChecked::Cacheable(CachePolicyCacheable {
                    result: self.result,
                    adapter: self.adapter,
                })
            }
            CachePolicy::NonCacheable(_) => {
                trace!("CachePolicyNonCacheable");
                CachePolicyChecked::NonCacheable(CachePolicyNonCacheable {
                    result: self.result,
                })
            }
        }
    }

    #[instrument]
    /// Store the value in the cache.
    pub async fn update_cache(self) -> CacheUpdated<A, T> {
        let cached_value = CachedValue::from((self.result, self.adapter.eviction_settings()));
        let cache_update_result = self.adapter.update_cache(&cached_value).await;
        if let Err(error) = cache_update_result {
            warn!("Updating cache error: {}", error.to_string())
        };
        trace!("CacheUpdated");
        CacheUpdated {
            adapter: self.adapter,
            result: cached_value.into_inner(),
        }
    }
}
