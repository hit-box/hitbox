use crate::CacheConfig;
use std::{fmt::Debug, sync::Arc};

use hitbox::{backend::CacheBackend, fsm::CacheFuture};
use hitbox_http::{CacheableHttpRequest, CacheableHttpResponse, FromBytes};
use http::{Request, Response};
use hyper::body::{Body, HttpBody};
use tower::Service;

use crate::future::Transformer;

pub struct CacheService<S, B, C> {
    upstream: S,
    backend: Arc<B>,
    configuration: C,
}

impl<S, B, C> CacheService<S, B, C> {
    pub fn new(upstream: S, backend: Arc<B>, configuration: C) -> Self {
        CacheService {
            upstream,
            backend,
            configuration,
        }
    }
}

impl<S, B, C> Clone for CacheService<S, B, C>
where
    S: Clone,
    B: Clone,
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            upstream: self.upstream.clone(),
            backend: self.backend.clone(),
            configuration: self.configuration.clone(),
        }
    }
}

impl<S, B, C, ReqBody, ResBody> Service<Request<ReqBody>> for CacheService<S, B, C>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    B: CacheBackend + Clone + Send + Sync + 'static,
    S::Future: Send,
    C: CacheConfig,

    // debug bounds
    ReqBody: Debug + HttpBody + Send + 'static,
    Body: From<ReqBody>,
    ResBody: FromBytes + HttpBody + Send + 'static,
    ResBody::Error: Debug,
    ResBody::Data: Send,
{
    type Response = Response<ResBody>;
    type Error = S::Error;
    type Future = CacheFuture<
        B,
        CacheableHttpRequest<ReqBody>,
        CacheableHttpResponse<ResBody>,
        Transformer<S, ReqBody>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.upstream.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let transformer = Transformer::new(self.upstream.clone());
        let configuration = &self.configuration;
        CacheFuture::new(
            self.backend.clone(),
            CacheableHttpRequest::from_request(req),
            transformer,
            Arc::new(configuration.request_predicates()),
            Arc::new(configuration.response_predicates()),
            Arc::new(configuration.extractors()),
            Arc::new(configuration.policy()),
        )
    }
}
