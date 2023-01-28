use std::{fmt::Debug, task::{Context, Poll}, sync::Arc};

use hitbox::dev::CacheBackend;
use hitbox_redis::RedisBackend;
use http::Request;
use tower::{Layer, Service};


pub struct CacheService<S, B> 
{
    upstream: S,
    backend: Arc<B>,
}

fn eater<B: CacheBackend>(backend: Arc<B>) { drop(backend) }

impl<S, T, B> Service<Request<T>> for CacheService<S, B>
where
    S: Service<Request<T>>,
    Request<T>: Debug,
    B: CacheBackend,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.upstream.poll_ready(cx)
    }

    fn call(&mut self, req: Request<T>) -> Self::Future {
        let a = self.backend.clone();
        eater(a);
        dbg!(&req);
        self.upstream.call(req)
    }
}

pub struct Cache<B> {
    backend: Arc<B>,
}

impl<S, B> Layer<S> for Cache<B>
{
    type Service = CacheService<S, B>;

    fn layer(&self, upstream: S) -> Self::Service {
        CacheService {
            upstream,
            backend: self.backend.clone(),
        }
    }
}

impl<B> Cache<B>
where
    B: CacheBackend,
{
    pub fn builder() -> CacheBuilder<RedisBackend> {
        CacheBuilder::default()
    }
}

pub struct CacheBuilder<B>
where
    B: CacheBackend,
{
    backend: Option<B>,
}

impl<B> CacheBuilder<B>
where
    B: CacheBackend,
{
    pub fn build(self) -> Cache<B> {
        Cache {
            backend: Arc::new(self.backend.unwrap()),
        }
    }
}

impl Default for CacheBuilder<RedisBackend> {
    fn default() -> Self {
        Self {
            backend: Some(RedisBackend::new().unwrap()),
        }
    }
}
