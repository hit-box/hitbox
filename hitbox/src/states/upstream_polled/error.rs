use std::fmt;

use tracing::{instrument, trace};

use crate::states::finish::Finish;
use crate::CacheError;

/// This state is a variant without data.
pub struct UpstreamPolledError {
    /// Returned error.
    pub error: CacheError,
}

/// Required `Debug` implementation to use `instrument` macro.
impl fmt::Debug for UpstreamPolledError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UpstreamPolledError")
    }
}

impl UpstreamPolledError {
    #[instrument]
    /// Upstream returns an error. FSM goes to Finish.
    pub fn finish<T>(self) -> Finish<T> {
        trace!("Finish");
        Finish {
            result: Err(self.error),
        }
    }
}
