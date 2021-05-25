use std::fmt;

use tracing::{instrument, trace};

use crate::CacheError;

/// Finite state.
pub struct Finish<T> {
    /// The field represents the return value.
    pub result: Result<T, CacheError>,
}

/// Required `Debug` implementation to use `instrument` macro.
impl<T> fmt::Debug for Finish<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Finish")
    }
}

impl<T> Finish<T> {
    #[instrument]
    /// Return inner value `result`.
    pub fn result(self) -> Result<T, CacheError> {
        trace!("Result");
        self.result
    }
}
