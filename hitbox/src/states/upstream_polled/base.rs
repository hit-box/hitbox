use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::states::upstream_polled::{UpstreamPolledSuccessful, UpstreamPolledError, UpstreamPolledErrorStaleRetrieved};

pub enum UpstreamPolled<A, T>
where
    A: RuntimeAdapter,
{
    Successful(UpstreamPolledSuccessful<A, T>),
    Error(UpstreamPolledError),
}

pub enum UpstreamPolledStaleRetrieved<A, T>
where
    A: RuntimeAdapter,
{
    Successful(UpstreamPolledSuccessful<A, T>),
    Error(UpstreamPolledErrorStaleRetrieved<T>),
}