use std::fmt;

use tracing::{instrument, trace};

use crate::states::finish::Finish;

/// This state is a non cacheable variant from [CachePolicyChecked](enum.CachePolicyChecked.html).
pub struct CachePolicyNonCacheable<T> {
    /// Value retrieved from upstream.
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
