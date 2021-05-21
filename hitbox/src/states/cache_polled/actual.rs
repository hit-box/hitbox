use std::fmt;
use std::fmt::Debug;

use tracing::{instrument, trace, warn};

use crate::response::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;
use crate::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};
use crate::CachedValue;

/// This state is a variant with actual data from [CachePolled](enum.CachePolled.html).
pub struct CachePolledActual<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    /// Runtime adapter.
    pub adapter: A,
    /// Value retrieved from cache.
    pub result: CachedValue<T>,
}

/// Required `Debug` implementation to use `instrument` macro.
impl<A, T> fmt::Debug for CachePolledActual<A, T>
where
    A: RuntimeAdapter,
    T: CacheableResponse,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CachePolledActual")
    }
}

impl<A, T> CachePolledActual<A, T>
where
    A: RuntimeAdapter,
    T: Debug + CacheableResponse,
{
    #[instrument]
    /// We have to return actual data.
    pub fn finish(self) -> Finish<T> {
        trace!("Finish");
        Finish {
            result: Ok(self.result.into_inner()),
        }
    }
}
