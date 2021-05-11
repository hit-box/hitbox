use std::fmt::Debug;

use tracing::trace;

use crate::CacheError;
use crate::states::finish::Finish;

pub struct UpstreamPolledError {
    pub error: CacheError,
}

impl UpstreamPolledError {
    pub fn finish<T: Debug>(self) -> Finish<T> {
        trace!("-> Finish");
        Finish {
            result: Err(self.error),
        }
    }
}
