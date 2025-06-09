use crate::backend::{Expiration, MokaBackend};
use hitbox::{CacheKey, CacheValue};
use hitbox_backend::serializer::Raw;
use moka::future::{Cache, CacheBuilder};

pub struct MokaBackendBuilder {
    builder: CacheBuilder<CacheKey, CacheValue<Raw>, Cache<CacheKey, CacheValue<Raw>>>,
}

impl MokaBackendBuilder {
    pub fn new(max_capacity: u64) -> Self {
        let builder = CacheBuilder::new(max_capacity);
        Self { builder }
    }

    pub fn build(self) -> MokaBackend {
        let expiry = Expiration;
        let cache = self.builder.expire_after(expiry).build();
        MokaBackend { cache }
    }
}
