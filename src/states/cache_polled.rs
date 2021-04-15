use crate::adapted::runtime_adapter::RuntimeAdapter;
use crate::states::finish::Finish;
use std::fmt::Debug;
use crate::CacheError;
use crate::states::upstream_polled::{UpstreamPolled, UpstreamPolledSuccessful, UpstreamPolledError};

pub struct CachePolledSuccessful<A, T>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
    pub result: T
}

impl<A, T> CachePolledSuccessful<A, T>
where
    A: RuntimeAdapter,
    T: Debug,
{
    pub fn finish(self) -> Finish<T> {
        Finish { result: self.result }
    }
}

// pub struct CachePolledError<A, T> {
//     pub error: CacheError
// }
//
// impl CachePolledError {
//     pub fn poll_upstream(self) -> UpstreamPolled<A, T>
//     where
//         A: RuntimeAdapter<UpstreamResult = T>
//     {
//         match self.adapter.poll_upstream().await {
//             Ok(result) => UpstreamPolled::Successful(
//                 UpstreamPolledSuccessful { adapter: self.adapter, result }
//             ),
//             Err(error) => UpstreamPolled::Error(UpstreamPolledError { error }),
//         }
//     }
// }

pub enum CachePolled<A, T>
where
    A: RuntimeAdapter,
{
    Successful(CachePolledSuccessful<A, T>),
    // Error(CachePolledError),
}
