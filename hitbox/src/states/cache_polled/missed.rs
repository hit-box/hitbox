use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::states::upstream_polled::{UpstreamPolled, UpstreamPolledSuccessful, UpstreamPolledError};

/// Cache miss state.
pub struct CacheMissed<A>
    where
        A: RuntimeAdapter,
{
    pub adapter: A,
}

impl<A> CacheMissed<A>
where
    A: RuntimeAdapter,
{
    pub async fn poll_upstream<T>(self) -> UpstreamPolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>
    {
        match self.adapter.poll_upstream().await {
            Ok(result) => UpstreamPolled::Successful(
                UpstreamPolledSuccessful { adapter: self.adapter, result }
            ),
            Err(error) => UpstreamPolled::Error(UpstreamPolledError { error }),
        }
    }
}
