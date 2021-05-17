use std::fmt;

use tracing::{instrument, trace};

use crate::states::finish::Finish;

/// State represents non cacheable policy option.
///
/// The field result of the `CachePolicyCacheable` structure is a value
/// that is retrieved from the upstream (DB or something similar) and won't be cached.
pub struct CachePolicyNonCacheable<T> {
    pub result: T,
}

/// Required `Debug` implementation to use `instrument` macro.
impl<T> fmt::Debug for CachePolicyNonCacheable<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CachePolicyNonCacheable")
    }
}

impl<T> CachePolicyNonCacheable<T> {
    #[instrument]
    /// If the value cannot be cached, we have to return it.
    pub fn finish(self) -> Finish<T> {
        trace!("Finish");
        Finish {
            result: Ok(self.result),
        }
    }
}
