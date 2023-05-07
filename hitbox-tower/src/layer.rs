use std::{marker::PhantomData, sync::Arc};

use hitbox::dev::CacheBackend;
use hitbox_redis::RedisBackend;
use tower::Layer;

use crate::service::CacheService;

#[derive(Clone)]
pub struct Cache<B> {
    pub backend: Arc<B>,
    pub config: String,
}

impl<B> Cache<B> {
    pub fn new(backend: B) -> Cache<B> {
        Cache {
            backend: Arc::new(backend),
            config: "Config".to_string(),
        }
    }
}

impl<S, B> Layer<S> for Cache<B> {
    type Service = CacheService<S, B>;

    fn layer(&self, upstream: S) -> Self::Service {
        CacheService::new(upstream, Arc::clone(&self.backend))
    }
}

impl Cache<RedisBackend> {
    pub fn builder() -> CacheBuilder<RedisBackend> {
        CacheBuilder::<RedisBackend>::default()
    }
}

pub struct CacheBuilder<B> {
    backend: Option<B>,
}

impl<B> CacheBuilder<B>
where
    B: CacheBackend,
{
    pub fn backend<NB: CacheBackend>(self, backend: NB) -> CacheBuilder<NB> {
        CacheBuilder {
            backend: Some(backend),
        }
    }

    pub fn build(self) -> Cache<B> {
        Cache {
            backend: Arc::new(self.backend.expect("Please add some cache backend")),
        }
    }
}

impl<B> Default for CacheBuilder<B> {
    fn default() -> Self {
        Self { backend: None }
    }
}
