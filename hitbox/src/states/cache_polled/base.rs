use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::states::cache_polled::{
    CacheErrorOccurred, CacheMissed, CachePolledActual, CachePolledStale,
};

pub enum CachePolled<A, T>
where
    A: RuntimeAdapter,
{
    Actual(CachePolledActual<A, T>),
    Stale(CachePolledStale<A, T>),
    Miss(CacheMissed<A>),
    Error(CacheErrorOccurred<A>),
}
