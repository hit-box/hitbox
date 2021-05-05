use crate::CacheError;
use std::fmt::Debug;
use crate::states::finish::Finish;

pub struct UpstreamPolledError {
    pub error: CacheError
}

impl UpstreamPolledError {
    pub fn finish<T: Debug>(self) -> Finish<T> {
        Finish { result: Err(self.error) }
    }
}
