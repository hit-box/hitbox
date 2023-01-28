use std::{
    fmt::Debug,
    sync::Arc,
    task::{Context, Poll},
};

use hitbox::dev::CacheBackend;
use hitbox_redis::RedisBackend;
use http::Request;
use tower::{Layer, Service};

pub struct CacheService<S, B> {
    upstream: S,
    backend: Arc<B>,
}

impl<S, B> Clone for CacheService<S, B>
where
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            upstream: self.upstream.clone(),
            backend: Arc::clone(&self.backend),
        }
    }
}

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
        drop(a);
        dbg!(&req);
        self.upstream.call(req)
    }
}

pub struct Cache<B> {
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
            backend: Arc::new(self.backend.unwrap()),
        }
    }
}

impl<B> Default for CacheBuilder<B> {
    fn default() -> Self {
        Self { backend: None }
    }
}
