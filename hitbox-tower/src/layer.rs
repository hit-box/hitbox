use std::sync::Arc;

use hitbox::dev::CacheBackend;
use hitbox_redis::RedisBackend;
use tower::Layer;

use crate::service::CacheService;

pub struct Cache<B = RedisBackend> {
    backend: Arc<B>,
}

impl<B> Clone for Cache<B> {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
        }
    }
}

impl<S, B> Layer<S> for Cache<B> {
    type Service = CacheService<S, B>;

    fn layer(&self, upstream: S) -> Self::Service {
        CacheService::new(upstream, self.backend.clone())
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
    pub fn backend(mut self, backend: B) -> Self {
        self.backend = Some(backend);
        self
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