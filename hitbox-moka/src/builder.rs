use crate::backend::{Expiration, MokaBackend};
use hitbox::{CacheKey, CacheValue};
use hitbox_backend::{CacheKeyFormat, Compressor, PassthroughCompressor};
use hitbox_backend::serializer::{Format, Raw};
use moka::future::{Cache, CacheBuilder};

pub struct MokaBackendBuilder<C: Compressor = PassthroughCompressor> {
    builder: CacheBuilder<CacheKey, CacheValue<Raw>, Cache<CacheKey, CacheValue<Raw>>>,
    key_format: CacheKeyFormat,
    value_format: Format,
    compressor: C,
}

impl MokaBackendBuilder<PassthroughCompressor> {
    pub fn new(max_capacity: u64) -> Self {
        let builder = CacheBuilder::new(max_capacity);
        Self {
            builder,
            key_format: CacheKeyFormat::Bitcode,
            value_format: Format::Json,
            compressor: PassthroughCompressor,
        }
    }
}

impl<C: Compressor> MokaBackendBuilder<C> {
    pub fn key_format(mut self, format: CacheKeyFormat) -> Self {
        self.key_format = format;
        self
    }

    pub fn value_format(mut self, format: Format) -> Self {
        self.value_format = format;
        self
    }

    pub fn compressor<NewC: Compressor>(self, compressor: NewC) -> MokaBackendBuilder<NewC> {
        MokaBackendBuilder {
            builder: self.builder,
            key_format: self.key_format,
            value_format: self.value_format,
            compressor,
        }
    }

    pub fn build(self) -> MokaBackend<C> {
        let expiry = Expiration;
        let cache = self.builder.expire_after(expiry).build();
        MokaBackend {
            cache,
            key_format: self.key_format,
            value_format: self.value_format,
            compressor: self.compressor,
        }
    }
}
