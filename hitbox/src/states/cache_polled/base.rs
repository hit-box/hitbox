use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::cache_polled::{
    CacheErrorOccurred, CacheMissed, CachePolledActual, CachePolledStale,
};

/// Enum represents all possible cache states.
pub enum CachePolled<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    /// Cache found, ttl and stale ttl not expired.
    Actual(CachePolledActual<A, T>),
    /// Cache found, stale ttl expired.
    Stale(CachePolledStale<A, T>),
    /// Cache not found.
    Miss(CacheMissed<A>),
    /// Unable to get cache from [Backend].
    Error(CacheErrorOccurred<A>),
}
