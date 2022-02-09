use crate::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;
use crate::states::initial::Initial;
use crate::states::upstream_polled::UpstreamPolled;
use std::fmt::Debug;

/// Transition for `InitialCacheSettings::CacheDisabled` option.
pub async fn transition<T, A>(state: Initial<A>) -> Finish<T>
where
    A: RuntimeAdapter,
    A: RuntimeAdapter<UpstreamResult = T>,
    T: Debug + CacheableResponse,
{
    match state.poll_upstream().await {
        UpstreamPolled::Successful(state) => state.finish(),
        UpstreamPolled::Error(error) => error.finish(),
    }
}
