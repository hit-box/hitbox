use crate::CacheError;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Finish<T: Debug> {
    pub result: Result<T, CacheError>,
}

impl<T> Finish<T>
where
    T: Debug,
{
    pub fn result(self) -> Result<T, CacheError> {
        self.result
    }
}
