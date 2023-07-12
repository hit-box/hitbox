use crate::error::Error;
use std::time::Duration;
use stretto::AsyncCacheBuilder;

type Cache = AsyncCacheBuilder<String, Vec<u8>>;

pub struct StrettoBackendBuilder(Cache);

impl StrettoBackendBuilder {
    pub fn new(num_counters: usize, max_cost: i64) -> Self {
        Self(AsyncCacheBuilder::new(num_counters, max_cost))
    }

    pub fn set_buffer_size(self, sz: usize) -> Self {
        Self(self.0.set_buffer_size(sz))
    }

    pub fn set_buffer_items(self, sz: usize) -> Self {
        Self(self.0.set_buffer_items(sz))
    }

    pub fn set_ingore_internal_cost(self, val: bool) -> Self {
        Self(self.0.set_ignore_internal_cost(val))
    }

    pub fn set_cleanup_duration(self, d: Duration) -> Self {
        Self(self.0.set_cleanup_duration(d))
    }

    pub fn finalize(self) -> Result<crate::backend::StrettoBackend, Error> {
        self.0
            .finalize(tokio::spawn)
            .map(crate::backend::StrettoBackend::new)
            .map_err(Error::from)
    }
}
