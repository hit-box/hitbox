use std::fmt;
use std::fmt::Debug;

use tracing::{instrument, trace};

use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;

/// State after transition `update_cache`.
///
/// The transition to this state doesn't depend on the success of the cache update operation.
pub struct CacheUpdated<A, T>
where
    A: RuntimeAdapter,
{
    /// Runtime adapter.
    pub adapter: A,
    /// Value retrieved from cache or from upstream.
    pub result: T,
}

/// Required `Debug` implementation to use `instrument` macro.
impl<A, T> fmt::Debug for CacheUpdated<A, T>
where
    A: RuntimeAdapter,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CacheUpdated")
    }
}

impl<A, T> CacheUpdated<A, T>
where
    A: RuntimeAdapter,
    T: Debug,
{
    #[instrument]
    /// We have to return actual data.
    pub fn finish(self) -> Finish<T> {
        trace!("Finish");
        Finish {
            result: Ok(self.result),
        }
    }
}
