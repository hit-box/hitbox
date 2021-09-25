use crate::config::CacheConfig;
use crate::service::CacheService;
use tower_layer::Layer;

#[derive(Debug, Default)]
pub struct CacheLayer {
    cache_config: CacheConfig,
}

impl CacheLayer {
    pub fn build() -> CacheLayerBuilder {
        CacheLayerBuilder::default()
    }
}

impl<S> Layer<S> for CacheLayer {
    type Service = CacheService<S>;

    fn layer(&self, service: S) -> Self::Service {
        CacheService::new(service, self.cache_config.clone())
    }
}

#[derive(Debug, Default)]
pub struct CacheLayerBuilder {
    key_prefix: Option<String>,
    ttl: Option<u32>,
    stale_ttl: Option<u32>,
    version: Option<u32>,
}

impl CacheLayerBuilder {
    pub fn key_prefix(mut self, prefix: &str) -> CacheLayerBuilder {
        self.key_prefix = Some(prefix.to_string());
        self
    }
    pub fn ttl(mut self, ttl: u32) -> CacheLayerBuilder {
        self.ttl = Some(ttl);
        self
    }
    pub fn stale_ttl(mut self, stale_ttl: u32) -> CacheLayerBuilder {
        self.stale_ttl = Some(stale_ttl);
        self
    }
    pub fn version(mut self, version: u32) -> CacheLayerBuilder {
        self.version = Some(version);
        self
    }

    pub fn finish(self) -> CacheLayer {
        CacheLayer {
            cache_config: CacheConfig {
                key_prefix: self.key_prefix,
                ttl: self.ttl,
                stale_ttl: self.stale_ttl,
                version: self.version,
            },
        }
    }
}
