use std::{fmt::Debug, marker::PhantomData, pin::Pin, sync::Arc};

use chrono::{Duration, Utc};
use futures::{
    future::{BoxFuture, Map},
    Future, FutureExt,
};
use hitbox::{
    backend::{BackendError, CacheBackend},
    Cacheable, CachedValue,
};
use hitbox_backend::{
    CacheableResponse, CacheableResponseWrapper,
    Backend,
};
use hitbox_http::{CacheableRequest, HttpResponse, SerializableHttpResponse};
use http::{Request, Response};
use serde::{de::DeserializeOwned, Serialize};
use tower::Service;

use hitbox::fsm::CacheFuture;

pub struct CacheService<S, B> {
    upstream: S,
    backend: Arc<B>,
}

impl<S, B> CacheService<S, B> {
    pub fn new(upstream: S, backend: Arc<B>) -> Self {
        CacheService { upstream, backend }
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

impl<S, Body, B, Res, C> Service<Request<Body>> for CacheService<S, B>
where
    S: Service<Request<Body>, Response = Response<Res>>,
    B: CacheBackend + Send + Sync + Clone + 'static,
    S::Future: Send,
    S::Error: Send + Sync + Debug + 'static,
    Body: Send,
    Res: Send + Debug + 'static,
    Request<Body>: Debug,
    HttpResponse<Res>:
        CacheableResponseWrapper<Source = <S::Future as Future>::Output> + CacheableResponse<Cached = C>,
    C: Debug + Serialize + DeserializeOwned + Send + Clone,
{
    type Response = Response<Res>;
    type Error = S::Error;
    type Future = CacheFuture<S::Future, B, HttpResponse<Res>>;

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

        let upstream = self.upstream.call(req);
        // .map(Box::new(|res| HttpResponse::new(res.unwrap())));
        // Box::pin(CacheFuture::new(upstream, self.backend.clone(), cache_key).map(|res| res.into_response()))
        // CacheFutureWrapper::new(upstream, backend, cache_key)
        // new(upstream, backend, cache_key)
        CacheFuture::new(upstream, self.backend.clone(), cache_key)
    }
}
