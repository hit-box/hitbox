use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::states::cache_polled::CachePolled;
use crate::states::finish::Finish;
use crate::states::initial::InitialState;
use crate::states::upstream_polled::{UpstreamPolled, UpstreamPolledStaleRetrieved};
use crate::CacheError;
use std::fmt::Debug;

pub async fn transition<T, A>(state: InitialState<A>) -> Finish<T>
where
    A: RuntimeAdapter,
    A: RuntimeAdapter<UpstreamResult = T>,
    T: Debug,
{
    match state.poll_cache().await {
        CachePolled::Actual(state) => state.finish(),
        CachePolled::Stale(state) => match state.poll_upstream().await {
            UpstreamPolledStaleRetrieved::Successful(state) => state.update_cache().await.finish(),
            UpstreamPolledStaleRetrieved::Error(state) => state.finish(),
        },
        CachePolled::Miss(state) => match state.poll_upstream().await {
            UpstreamPolled::Successful(state) => state.update_cache().await.finish(),
            UpstreamPolled::Error(error) => error.finish(),
        },
        CachePolled::Error(state) => match state.poll_upstream().await {
            UpstreamPolled::Successful(state) => state.update_cache().await.finish(),
            UpstreamPolled::Error(error) => error.finish(),
        },
    }
}
