use std::fmt::Debug;

use tracing::trace;

use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;

pub struct CacheUpdated<A, T>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
    pub result: T,
}

impl<A, T> CacheUpdated<A, T>
where
    A: RuntimeAdapter,
    T: Debug,
{
    pub fn finish(self) -> Finish<T> {
        trace!("-> Finish");
        Finish {
            result: Ok(self.result),
        }
    }
}
