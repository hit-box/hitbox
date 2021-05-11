use std::fmt;

use tracing::{instrument, trace};

use crate::states::finish::Finish;

pub struct CachePolicyNonCacheable<T> {
    pub result: T,
}

impl<T> fmt::Debug for CachePolicyNonCacheable<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CachePolicyNonCacheable")
    }
}

impl<T> CachePolicyNonCacheable<T> {
    pub fn finish(self) -> Finish<T> {
        trace!("Finish");
        Finish {
            result: Ok(self.result),
        }
    }
}
