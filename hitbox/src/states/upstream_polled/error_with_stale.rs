use std::fmt::Debug;

use tracing::trace;

use crate::CacheError;
use crate::states::finish::Finish;

pub struct UpstreamPolledErrorStaleRetrieved<T> {
    pub error: CacheError,
    pub result: T,
}

impl<T> UpstreamPolledErrorStaleRetrieved<T>
where
    T: Debug,
{
    pub fn finish(self) -> Finish<T> {
        trace!("-> Finish");
        Finish {
            result: Ok(self.result),
        }
    }
}
