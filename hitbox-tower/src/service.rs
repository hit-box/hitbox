use crate::config::Config;
use std::{fmt::Debug, sync::Arc};

use hitbox::{backend::CacheBackend, fsm::CacheFuture};
use hitbox_http::{
    extractors::NeutralExtractor,
    extractors::{method::MethodExtractor, path::PathExtractor},
    predicates::{query::QueryPredicate, NeutralPredicate, NeutralResponsePredicate},
    CacheableHttpRequest, CacheableHttpResponse, FromBytes,
};
use http::{Request, Response};
use hyper::body::{Body, HttpBody};
use tower::Service;

use crate::future::Transformer;

pub struct CacheService<S, B> {
    upstream: S,
    backend: Arc<B>,
    config: Config,
}

impl<S, B> CacheService<S, B> {
    pub fn new(upstream: S, backend: Arc<B>, config: Config) -> Self {
        CacheService { upstream, backend, config }
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
            config: self.config.clone(),
        }
    }
}

impl<S, B, ReqBody, ResBody> Service<Request<ReqBody>> for CacheService<S, B>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    B: CacheBackend + Clone + Send + Sync + 'static,
    S::Future: Send,

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
        dbg!(&req);

        let transformer = Transformer::new(self.upstream.clone());
        let config = self.config.clone();
        let request_predicate = NeutralPredicate::new().query(config.query.0, config.query.1);
        let response_predicate = NeutralResponsePredicate::new();
        let extractor = NeutralExtractor::new().method().path("/{path}*");
        CacheFuture::new(
            self.backend.clone(),
            CacheableHttpRequest::from_request(req),
            transformer,
            Arc::new(request_predicate),
            Arc::new(response_predicate),
            Arc::new(extractor),
        )
    }
}
