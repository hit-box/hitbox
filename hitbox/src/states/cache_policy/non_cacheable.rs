use crate::runtime::RuntimeAdapter;
use crate::response::CacheableResponse;
use std::fmt::Debug;
use crate::states::finish::Finish;

pub struct CachePolicyNonCacheable<T: Debug> {
    pub result: T,
}

impl<T: Debug> CachePolicyNonCacheable<T> {
    pub fn finish(self) -> Finish<T> {
        Finish { result: Ok(self.result) }
    }
}
