use crate::states::finish::Finish;
use crate::CacheError;
use std::fmt::Debug;

pub struct UpstreamPolledErrorStaleRetrieved<T> {
    pub error: CacheError,
    pub result: T,
}

impl<T> UpstreamPolledErrorStaleRetrieved<T>
where
    T: Debug,
{
    pub fn finish(self) -> Finish<T> {
        Finish {
            result: Ok(self.result),
        }
    }
}
