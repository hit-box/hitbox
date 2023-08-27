use crate::{error::Error, StrettoBackend};
use hitbox::CacheKey;
use std::time::Duration;
use stretto::AsyncCacheBuilder;

type Cache = AsyncCacheBuilder<CacheKey, Vec<u8>>;

pub struct StrettoBackendBuilder(Cache);

impl StrettoBackendBuilder {
    pub fn new(max_size: i64) -> Self {
        let num_counters = max_size * 10;
        Self(AsyncCacheBuilder::new(num_counters as usize, max_size))
    }

    pub fn set_buffer_size(self, sz: usize) -> Self {
        Self(self.0.set_buffer_size(sz))
    }

    pub fn set_buffer_items(self, sz: usize) -> Self {
        Self(self.0.set_buffer_items(sz))
    }

    pub fn set_cleanup_duration(self, d: Duration) -> Self {
        Self(self.0.set_cleanup_duration(d))
    }

    pub fn finalize(self) -> Result<StrettoBackend, Error> {
        self.0
            .set_ignore_internal_cost(true)
            .finalize(tokio::spawn)
            .map(|cache| StrettoBackend { cache })
            .map_err(Error::from)
    }
}
