use crate::states::initial::InitialState;
use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::states::cache_polled::CachePolled;
use crate::states::upstream_polled::UpstreamPolled;
use crate::states::finish::Finish;
use std::fmt::Debug;
use crate::CacheError;
use crate::states::stale_upstream_polled::StaleUpstreamPolled;


pub async fn transition<T, A>(state: InitialState<A>) -> Finish<T>
where
    A: RuntimeAdapter,
    A: RuntimeAdapter<UpstreamResult = T>,
    T: Debug,
{
    match state.poll_cache().await {
        CachePolled::Actual(state) => state.finish(),
        CachePolled::Stale(state) => {
            match state.poll_upstream().await {
                StaleUpstreamPolled::Successful(state) => state.update_cache().await.finish(),
                StaleUpstreamPolled::Error(state) => state.finish(),
            }
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
