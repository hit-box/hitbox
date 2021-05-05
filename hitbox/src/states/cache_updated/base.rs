use crate::adapted::runtime_adapter::RuntimeAdapter;
use std::fmt::Debug;
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
        Finish { result: Ok(self.result) }
    }
}