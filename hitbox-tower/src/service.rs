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
use hitbox_backend::{CacheableResponse, CacheableResponseWrapper};
use hitbox_http::{CacheableRequest, HttpResponse, SerializableHttpResponse};
use http::{Request, Response};
use hyper::Body;
use serde::{de::DeserializeOwned, Serialize};
use tower::Service;

use hitbox::fsm::CacheFuture;

use crate::future::CacheFutureAdapter;

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

impl<S, B, Res, C> Service<Request<Body>> for CacheService<S, B>
where
    S: Service<Request<Body>, Response = Response<Res>> + Clone,
    B: CacheBackend + Send + Sync + Clone + 'static,
    S::Future: Send,
    S::Error: Send + Sync + Debug + 'static,
    Body: Send,
    Res: Send + Debug + 'static,
    Request<Body>: Debug,
    HttpResponse<Res>: CacheableResponseWrapper<Source = <S::Future as Future>::Output>
        + CacheableResponse<Cached = C>,
    C: Debug + Serialize + DeserializeOwned + Send + Clone,
{
    type Response = Response<Res>;
    type Error = S::Error;
    // type Future = CacheFuture<S::Future, B, HttpResponse<Res>, CacheableRequest<Body>>;
    type Future = CacheFutureAdapter<S::Future, S>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.upstream.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        dbg!(&req);
        // let upstream = async move {
        //     let cacheable_request = CacheableRequest::from_request(req);
        //     let policy = cacheable_request.cache_key().await;
        //     self.upstream.call(cacheable_request.into_origin())
        // };
        // let cache_key = cacheable_request.cache_key().unwrap();
        // dbg!(&cache_key);

        // CacheFuture::new(upstream, self.backend.clone(), cacheable_request)
        CacheFutureAdapter::new(&mut self.upstream, req)
    }
}
