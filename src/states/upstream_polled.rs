use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::states::finish::Finish;
use std::fmt::Debug;
use crate::CacheError;

pub struct UpstreamPolledSuccessful<A, T>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
    pub result: T
}

impl<A, T> UpstreamPolledSuccessful<A, T>
where
    A: RuntimeAdapter,
    T: Debug,
{
    pub fn finish(self) -> Finish<T> {
        Finish { result: self.result }
    }
}

pub struct UpstreamPolledError {
    pub error: CacheError
}

impl UpstreamPolledError {
    pub fn finish(self) -> Finish<CacheError> {
        Finish { result: self.error }
    }
}

pub enum UpstreamPolled<A, T>
where
    A: RuntimeAdapter,
{
    Successful(UpstreamPolledSuccessful<A, T>),
    Error(UpstreamPolledError),
}
