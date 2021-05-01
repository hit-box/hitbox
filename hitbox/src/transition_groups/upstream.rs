use crate::states::initial::InitialState;
use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::states::upstream_polled::UpstreamPolled;
use crate::states::finish::Finish;
use std::fmt::Debug;
use crate::CacheError;

pub async fn transition<T, A>(state: InitialState<A>) -> Result<T, CacheError>
where
    A: RuntimeAdapter,
    A: RuntimeAdapter<UpstreamResult = T>,
    T: Debug,
{
    match state.poll_upstream().await {
        UpstreamPolled::Successful(state) => Ok(state.finish().result()),
        UpstreamPolled::Error(error) => Err(error.finish().result()),
    }
}

