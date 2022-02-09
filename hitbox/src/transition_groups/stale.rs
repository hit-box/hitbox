use crate::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::cache_policy::CachePolicyChecked;
use crate::states::cache_polled::CachePolled;
use crate::states::finish::Finish;
use crate::states::initial::Initial;
use crate::states::upstream_polled::{UpstreamPolled, UpstreamPolledStaleRetrieved};
use std::fmt::Debug;

/// Transition for `InitialCacheSettings::CacheStale` option.
pub async fn transition<T, A>(state: Initial<A>) -> Finish<T>
where
    A: RuntimeAdapter,
    A: RuntimeAdapter<UpstreamResult = T>,
    T: Debug + CacheableResponse,
{
    match state.poll_cache().await {
        CachePolled::Actual(state) => state.finish(),
        CachePolled::Stale(state) => match state.poll_upstream().await {
            UpstreamPolledStaleRetrieved::Successful(state) => match state.check_cache_policy() {
                CachePolicyChecked::Cacheable(state) => state.update_cache().await.finish(),
                CachePolicyChecked::NonCacheable(state) => state.finish(),
            },
            UpstreamPolledStaleRetrieved::Error(state) => state.finish(),
        },
        CachePolled::Miss(state) => match state.poll_upstream().await {
            UpstreamPolled::Successful(state) => match state.check_cache_policy() {
                CachePolicyChecked::Cacheable(state) => state.update_cache().await.finish(),
                CachePolicyChecked::NonCacheable(state) => state.finish(),
            },
            UpstreamPolled::Error(error) => error.finish(),
        },
        CachePolled::Error(state) => match state.poll_upstream().await {
            UpstreamPolled::Successful(state) => state.update_cache().await.finish(),
            UpstreamPolled::Error(error) => error.finish(),
        },
    }
}
