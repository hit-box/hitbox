use crate::runtime::RuntimeAdapter;
use crate::states::upstream_polled::{
    UpstreamPolledError, UpstreamPolledErrorStaleRetrieved, UpstreamPolledSuccessful,
};

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
