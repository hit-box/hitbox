use log::warn;

use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::cache_updated::CacheUpdated;
use crate::CachedValue;

pub struct CachePolicyCacheable<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    pub adapter: A,
    pub result: T,
}

impl<A, T> CachePolicyCacheable<A, T>
where
    A: RuntimeAdapter<UpstreamResult = T>,
    T: CacheableResponse,
{
    pub async fn update_cache(self) -> CacheUpdated<A, T> {
        let cached_value = CachedValue::new(self.result, chrono::Utc::now());
        let cache_update_result = self.adapter.update_cache(&cached_value).await;
        if let Err(error) = cache_update_result {
            warn!("Updating cache error: {}", error.to_string())
        };
        CacheUpdated {
            adapter: self.adapter,
            result: cached_value.into_inner(),
        }
    }
}
