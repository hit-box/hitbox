use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;
use std::fmt::Debug;

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
        Finish {
            result: Ok(self.result),
        }
    }
}
