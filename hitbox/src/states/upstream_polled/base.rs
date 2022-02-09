use crate::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::upstream_polled::{
    UpstreamPolledError, UpstreamPolledErrorStaleRetrieved, UpstreamPolledSuccessful,
};

/// Enum represents all possible upstream states.
pub enum UpstreamPolled<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    /// Value successful polled.
    Successful(UpstreamPolledSuccessful<A, T>),
    /// Error happened.
    Error(UpstreamPolledError),
}

/// Enum represents all possible upstream states when a stale value was retrieved.
pub enum UpstreamPolledStaleRetrieved<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    /// Value successful polled.
    Successful(UpstreamPolledSuccessful<A, T>),
    /// Error happened.
    Error(UpstreamPolledErrorStaleRetrieved<T>),
}
