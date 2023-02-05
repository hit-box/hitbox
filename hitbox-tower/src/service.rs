use std::{fmt::Debug, pin::Pin, sync::Arc, marker::PhantomData};

use chrono::{Duration, Utc};
use futures::{future::{BoxFuture, Map}, Future};
use hitbox::{
    dev::{BackendError, CacheBackend},
    Cacheable, CachedValue,
};
use hitbox_backend::backend::Backend;
use hitbox_http::{CacheableRequest, CacheableResponse};
use http::{Request, Response};
use tower::Service;

use crate::state::CacheFuture;

pub struct CacheService<S, B> {
    upstream: S,
    backend: Arc<B>,
}

impl<S, B> CacheService<S, B> {
    pub fn new(upstream: S, backend: Arc<B>) -> Self {
        CacheService {
            upstream,
            backend,
        }
    }
}

impl<S, B> Clone for CacheService<S, B>
where
    S: Clone,
    B: Clone,
{
    fn clone(&self) -> Self {
        Self {
            upstream: self.upstream.clone(),
            backend: Arc::clone(&self.backend),
        }
    }
}

impl<S, Body, B, Res> Service<Request<Body>> for CacheService<S, B>
where
    S: Service<Request<Body>, Response = Response<Res>>,
    B: CacheBackend + Send + Sync + Clone + 'static,
    S::Future: Send,
    S::Error: Send + Debug + 'static,
    Body: Send,
    Res: Send + 'static,
    Request<Body>: Debug,
{
    type Response = Response<Res>;
    type Error = S::Error;
    type Future = CacheFuture<S::Future, B>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.upstream.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        dbg!(&req);
        let cacheable_request = CacheableRequest::from_request(&req);
        let cache_key = cacheable_request.cache_key().unwrap();
        dbg!(&cache_key);

        let upstream = self
            .upstream
            .call(req);
            // .map(CacheableResponse::from_response);
        // Box::pin(CacheFuture::new(upstream, self.backend.clone(), cache_key).map(|res| res.into_response()))
        // CacheFutureWrapper::new(upstream, backend, cache_key)
        // new(upstream, backend, cache_key)
        CacheFuture::new(upstream, self.backend.clone(), cache_key)
    }
}
