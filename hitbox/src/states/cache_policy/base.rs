use std::fmt::Debug;

use crate::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::cache_policy::{CachePolicyCacheable, CachePolicyNonCacheable};

/// Enum represents cacheable and non cacheable states.
/// For example: we don't cache `Err` option from `Result`
/// and cache `Ok`.
/// Please take a look at [CacheableResponse]
pub enum CachePolicyChecked<A, T>
where
    A: RuntimeAdapter,
    T: Debug + CacheableResponse,
{
    /// This variant should be stored in cache backend.
    Cacheable(CachePolicyCacheable<A, T>),
    /// This variant shouldn't be stored in the cache backend.
    NonCacheable(CachePolicyNonCacheable<T>),
}
