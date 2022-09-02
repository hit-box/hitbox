use std::fmt;
use std::fmt::Debug;

use tracing::{instrument, trace};

use crate::CacheableResponse;
use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;
use crate::CachedValue;
#[cfg(feature = "metrics")]
use crate::metrics::CACHE_HIT_COUNTER;

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
        #[cfg(feature = "metrics")]
        metrics::increment_counter!(
            CACHE_HIT_COUNTER.as_ref(),
            "upstream" => self.adapter.upstream_name(),
            "message" => self.adapter.message_name(),
        );
        Finish {
            result: Ok(self.result.into_inner()),
        }
    }
}
