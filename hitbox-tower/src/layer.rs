use crate::config::EndpointConfig;
use std::sync::Arc;

use hitbox::backend::CacheBackend;
use tower::Layer;

use crate::{dummy::DummyBackend, service::CacheService};

#[derive(Clone)]
pub struct Cache<B> {
    pub backend: Arc<B>,
    pub endpoint_config: Arc<EndpointConfig>,
}

impl<B> Cache<B> {
    pub fn new(backend: B) -> Cache<B> {
        Cache {
            backend: Arc::new(backend),
            endpoint_config: Arc::new(Default::default()),
        }
    }
}

impl<S, B> Layer<S> for Cache<B> {
    type Service = CacheService<S, B>;

    fn layer(&self, upstream: S) -> Self::Service {
        CacheService::new(
            upstream,
            Arc::clone(&self.backend),
            Arc::new(Default::default()),
        )
    }
}

impl Cache<DummyBackend> {
    pub fn builder() -> CacheBuilder<DummyBackend> {
        CacheBuilder::<DummyBackend>::default()
    }
}

pub struct CacheBuilder<B> {
    backend: Option<B>,
    endpoint_config: Option<EndpointConfig>,
}

impl<B> CacheBuilder<B>
where
    B: CacheBackend,
{
    pub fn backend<NB: CacheBackend>(self, backend: NB) -> CacheBuilder<NB> {
        CacheBuilder {
            backend: Some(backend),
            endpoint_config: self.endpoint_config,
        }
    }

    pub fn build(self) -> Cache<B> {
        Cache {
            backend: Arc::new(self.backend.expect("Please add some cache backend")),
            endpoint_config: Arc::new(self.endpoint_config.unwrap_or_default()),
        }
    }
}

impl<B> Default for CacheBuilder<B> {
    fn default() -> Self {
        Self {
            backend: None,
            endpoint_config: Default::default(),
        }
    }
}
