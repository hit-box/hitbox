use std::fmt;
use std::fmt::Debug;

use tracing::{instrument, trace};

use crate::runtime::RuntimeAdapter;
use crate::states::finish::Finish;

pub struct CacheUpdated<A, T>
where
    A: RuntimeAdapter,
{
    pub adapter: A,
    pub result: T,
}

impl<A, T> fmt::Debug for CacheUpdated<A, T>
where
    A: RuntimeAdapter,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CacheUpdated")
    }
}

impl<A, T> CacheUpdated<A, T>
where
    A: RuntimeAdapter,
    T: Debug,
{
    #[instrument]
    pub fn finish(self) -> Finish<T> {
        trace!("Finish");
        Finish {
            result: Ok(self.result),
        }
    }
}
