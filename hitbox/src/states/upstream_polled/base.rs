use crate::runtime::RuntimeAdapter;
use crate::states::upstream_polled::{
    UpstreamPolledError, UpstreamPolledErrorStaleRetrieved, UpstreamPolledSuccessful,
};
use crate::response::CacheableResponse;

pub enum UpstreamPolled<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse
{
    Successful(UpstreamPolledSuccessful<A, T>),
    Error(UpstreamPolledError),
}

pub enum UpstreamPolledStaleRetrieved<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse
{
    Successful(UpstreamPolledSuccessful<A, T>),
    Error(UpstreamPolledErrorStaleRetrieved<T>),
}
