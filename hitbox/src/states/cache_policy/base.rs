use crate::runtime::RuntimeAdapter;
use crate::response::CacheableResponse;
use crate::states::cache_policy::{CachePolicyCacheable, CachePolicyNonCacheable};
use std::fmt::Debug;

pub enum CachePolicyChecked<A, T>
where
    A: RuntimeAdapter,
    T: Debug + CacheableResponse
{
    Cacheable(CachePolicyCacheable<A, T>),
    NonCacheable(CachePolicyNonCacheable<T>),
}
