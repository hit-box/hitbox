use std::fmt;

use tracing::{instrument, trace};

use crate::states::finish::Finish;
use crate::CacheError;

/// Stale value was retrieved and poll upstream returned an error.
pub struct UpstreamPolledErrorStaleRetrieved<T> {
    /// Returned error.
    pub error: CacheError,
    /// Stale value retrieved from cache.
    pub result: T,
}

/// Required `Debug` implementation to use `instrument` macro.
impl<T> fmt::Debug for UpstreamPolledErrorStaleRetrieved<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UpstreamPolledErrorStaleRetrieved")
    }
}

impl<T> UpstreamPolledErrorStaleRetrieved<T> {
    #[instrument]
    /// Upstream returns an error. FSM goes to Finish.
    pub fn finish(self) -> Finish<T> {
        trace!("Finish");
        Finish {
            result: Ok(self.result),
        }
    }
}
