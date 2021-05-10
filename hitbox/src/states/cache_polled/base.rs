use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::cache_polled::{
    CacheErrorOccurred, CacheMissed, CachePolledActual, CachePolledStale,
};

pub enum CachePolled<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    Actual(CachePolledActual<A, T>),
    Stale(CachePolledStale<A, T>),
    Miss(CacheMissed<A>),
    Error(CacheErrorOccurred<A>),
}
