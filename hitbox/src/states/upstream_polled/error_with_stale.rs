use std::fmt;

use tracing::{instrument, trace};

use crate::CacheError;
use crate::states::finish::Finish;

pub struct UpstreamPolledErrorStaleRetrieved<T> {
    pub error: CacheError,
    pub result: T,
}

impl<T> fmt::Debug for UpstreamPolledErrorStaleRetrieved<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UpstreamPolledErrorStaleRetrieved")
    }
}

impl<T> UpstreamPolledErrorStaleRetrieved<T> {
    #[instrument]
    pub fn finish(self) -> Finish<T> {
        trace!("Finish");
        Finish {
            result: Ok(self.result),
        }
    }
}
