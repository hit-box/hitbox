use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::states::finish::Finish;
use crate::states::initial::InitialState;
use crate::states::upstream_polled::UpstreamPolled;
use std::fmt::Debug;

pub async fn transition<T, A>(state: InitialState<A>) -> Finish<T>
where
    A: RuntimeAdapter,
    A: RuntimeAdapter<UpstreamResult = T>,
    T: Debug,
{
    match state.poll_upstream().await {
        UpstreamPolled::Successful(state) => state.finish(),
        UpstreamPolled::Error(error) => error.finish(),
    }
}
