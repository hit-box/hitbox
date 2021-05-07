use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;
use crate::states::initial::InitialState;
use crate::states::upstream_polled::UpstreamPolled;
use std::fmt::Debug;
use crate::response::CacheableResponse;

pub async fn transition<T, A>(state: InitialState<A>) -> Finish<T>
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
