use std::{fmt::Debug, marker::PhantomData, pin::Pin, sync::Arc};

use chrono::{Duration, Utc};
use futures::{
    future::{BoxFuture, Map},
    Future, FutureExt,
};
use hitbox::{
    backend::{BackendError, CacheBackend},
    fsm::{CacheFuture, Transform},
    Cacheable, CachedValue,
};
use hitbox_backend::CacheableResponse;
use hitbox_http::{CacheableHttpRequest, CacheableHttpResponse, SerializableHttpResponse};
use http::{Request, Response};
use hyper::Body;
use serde::{de::DeserializeOwned, Serialize};
use tower::Service;

use hitbox::fsm::CacheFuture3;
use tracing::log::warn;

use crate::future::{Transformer, UpstreamFuture};

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

impl<S, B, ReqBody> Service<Request<ReqBody>> for CacheService<S, B>
where
    S: Service<Request<ReqBody>, Response = Response<Body>> + Clone + Send + 'static,
    B: CacheBackend + Clone,
    S::Future: Send,

    // debug bounds
    ReqBody: Debug + hyper::body::HttpBody + Send + 'static,
    Body: From<ReqBody>,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = CacheFuture<
        B,
        CacheableHttpRequest<ReqBody>,
        CacheableHttpResponse,
        Transformer<S, ReqBody>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.upstream.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        dbg!(&req);

        let transformer = Transformer::new(self.upstream.clone());
        CacheFuture::new(
            self.backend.clone(),
            CacheableHttpRequest::from_request(req),
            transformer,
        )
    }
}
