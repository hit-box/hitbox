use crate::states::FinishState;

#[derive(Debug)]
pub struct CacheUpdated<T> {
    pub cached: T,
}

impl<T> CacheUpdated<T> {
    pub fn finish(self) -> FinishState<T> {
        println!("-> Finish");
        FinishState {
            result: self.cached,
        }
    }
}
