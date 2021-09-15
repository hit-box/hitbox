use crate::service::CacheService;
use tower_layer::Layer;

#[derive(Debug, Clone)]
pub struct CacheLayer {}

impl CacheLayer {
    pub fn new() -> Self {
        CacheLayer {}
    }
}

impl Default for CacheLayer {
    fn default() -> Self {
        CacheLayer::new()
    }
}

impl<S> Layer<S> for CacheLayer {
    type Service = CacheService<S>;

    fn layer(&self, service: S) -> Self::Service {
        CacheService::new(service)
    }
}
