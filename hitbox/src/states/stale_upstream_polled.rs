use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::states::finish::Finish;
use std::fmt::Debug;
use crate::CacheError;
use crate::states::upstream_polled::UpstreamPolledSuccessful;

pub struct StaleUpstreamPolledError<T> {
    pub error: CacheError,
    pub result: T
}

impl<T> StaleUpstreamPolledError<T>
where
    T: Debug
{
    pub fn finish(self) -> Finish<T> {
        Finish { result: Ok(self.result) }
    }
}

pub enum StaleUpstreamPolled<A, T>
where
    A: RuntimeAdapter,
{
    Successful(UpstreamPolledSuccessful<A, T>),
    Error(StaleUpstreamPolledError<T>),
}
