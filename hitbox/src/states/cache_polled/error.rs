use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};

pub struct CacheErrorOccurred<A>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
}

impl<A> CacheErrorOccurred<A>
where
    A: RuntimeAdapter,
{
    pub async fn poll_upstream<T>(mut self) -> UpstreamPolled<A, T>
    where
        A: RuntimeAdapter<UpstreamResult = T>,
        T: CacheableResponse,
    {
        match self.adapter.poll_upstream().await {
            Ok(result) => UpstreamPolled::Successful(UpstreamPolledSuccessful {
                adapter: self.adapter,
                result,
            }),
            Err(error) => UpstreamPolled::Error(UpstreamPolledError { error }),
        }
    }
}
