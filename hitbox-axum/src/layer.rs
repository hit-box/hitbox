use crate::service::CacheService;
use tower_layer::Layer;
use crate::CacheableRequest;

#[derive(Debug, Clone, Default)]
pub struct CacheLayer {
    cache_key_prefix: Option<String>,
    ttl: Option<u32>,
    stale_ttl: Option<u32>,
    cache_version: Option<u32>,
}

impl CacheLayer {
    pub fn new() -> Self {
        CacheLayer::default()
    }
}

impl<S> Layer<S> for CacheLayer {
    type Service = CacheService<S>;

    fn layer(&self, service: S) -> Self::Service {
        CacheService::new(service)
    }
}
