use crate::runtime::RuntimeAdapter;
use crate::response::CacheableResponse;
use std::fmt::Debug;
use crate::states::cache_updated::CacheUpdated;

pub struct CachePolicyCacheable<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse
{
    pub adapter: A,
    pub result: T,
    pub serialized: Vec<u8>,
}

impl<A, T> CachePolicyCacheable<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse
{
    pub async fn update_cache(self) -> CacheUpdated<A, T> {
        CacheUpdated {
            adapter: self.adapter,
            result: self.result,
        }
    }
}
