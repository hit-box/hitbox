use crate::config::EndpointConfig;
use std::{fmt::Debug, sync::Arc};

use hitbox::{backend::CacheBackend, fsm::CacheFuture, policy::PolicyConfig};
use hitbox_http::{CacheableHttpRequest, CacheableHttpResponse, FromBytes};
use http::{Request, Response};
use hyper::body::{Body, HttpBody};
use tower::Service;

use crate::future::Transformer;

pub struct CacheService<S, B> {
    upstream: S,
    backend: Arc<B>,
    endpoint_config: Arc<EndpointConfig>,
    policy: Arc<PolicyConfig>,
}

impl<S, B> CacheService<S, B> {
    pub fn new(
        upstream: S,
        backend: Arc<B>,
        endpoint_config: Arc<EndpointConfig>,
        policy: Arc<PolicyConfig>,
    ) -> Self {
        CacheService {
            upstream,
            backend,
            endpoint_config,
            policy,
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
            endpoint_config: Arc::clone(&self.endpoint_config),
            policy: Arc::clone(&self.policy),
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
        //dbg!(&req);

        let transformer = Transformer::new(self.upstream.clone());
        let config = &self.endpoint_config;
        CacheFuture::new(
            self.backend.clone(),
            CacheableHttpRequest::from_request(req),
            transformer,
            Arc::new(config.request_predicates()),
            Arc::new(config.response_predicates()),
            Arc::new(config.extractors()),
            self.policy.clone(),
        )
    }
}
