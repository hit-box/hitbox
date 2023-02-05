use std::sync::Arc;

use hitbox::dev::CacheBackend;
use hitbox_redis::RedisBackend;
use tower::Layer;

use crate::service::CacheService;

#[derive(Clone)]
pub struct Cache<'a, B = RedisBackend> {
    backend: &'a B,
}

impl<'a, S, B> Layer<S> for Cache<'a, B>
where
    B: Clone + 'a,
{
    type Service = CacheService<'a, S, B>;

    fn layer(&self, upstream: S) -> Self::Service {
        CacheService::new(upstream, self.backend)
    }
}

impl<'a> Cache<'a, RedisBackend> {
    pub fn builder() -> CacheBuilder<'a, RedisBackend> {
        CacheBuilder::<RedisBackend>::default()
    }
}

pub struct CacheBuilder<'a, B> {
    backend: Option<&'a B>,
}

impl<'a, B> CacheBuilder<'a, B>
where
    B: CacheBackend,
{
    pub fn backend(mut self, backend: &'a B) -> Self {
        self.backend = Some(backend);
        self
    }

    pub fn build(self) -> Cache<'a, B> {
        Cache {
            backend: self
                .backend
                .as_ref()
                .expect("Please add some cache backend"),
        }
    }
}

impl<'a, B> Default for CacheBuilder<'a, B> {
    fn default() -> Self {
        Self { backend: None }
    }
}
