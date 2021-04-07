use crate::states::{FinishState, CacheUpdated};

#[derive(Debug)]
pub struct UpstreamPolled<T> {
    pub upstream_result: T,
}

impl<T> UpstreamPolled<T> {
    pub fn finish(self) -> FinishState<T> {
        println!("-> Finish");
        FinishState {
            result: self.upstream_result,
        }
    }

    pub fn update_cache(self) -> CacheUpdated<T> {
        println!("-> Update cache");
        CacheUpdated {
            cached: self.upstream_result,
        }
    }
}
