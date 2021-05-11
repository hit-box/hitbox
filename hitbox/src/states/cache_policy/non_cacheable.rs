use tracing::trace;
use crate::states::finish::Finish;
use std::fmt::Debug;

pub struct CachePolicyNonCacheable<T: Debug> {
    pub result: T,
}

impl<T: Debug> CachePolicyNonCacheable<T> {
    pub fn finish(self) -> Finish<T> {
        trace!("-> Finish");
        Finish {
            result: Ok(self.result),
        }
    }
}
