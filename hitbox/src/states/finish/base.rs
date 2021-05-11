use tracing::{trace, instrument};

use crate::CacheError;
use std::fmt::Debug;
use std::fmt;

pub struct Finish<T> {
    pub result: Result<T, CacheError>,
}

impl<T> fmt::Debug for Finish<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Finish")
    }
}

impl<T> Finish<T> {
    #[instrument]
    pub fn result(self) -> Result<T, CacheError> {
        trace!("Result");
        self.result
    }
}
