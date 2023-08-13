use std::sync::Arc;

use hitbox::{backend::CacheBackend, predicates::Predicate, Extractor};
use hitbox_http::{
    extractors::NeutralExtractor,
    extractors::{method::MethodExtractor, path::PathExtractor},
    predicates::{query::QueryPredicate, NeutralPredicate, NeutralResponsePredicate},
    CacheableHttpRequest, CacheableHttpResponse, FromBytes,
};
use http::{Request, Response};
use tower::Layer;

use crate::{dummy::DummyBackend, service::CacheService};

#[derive(Clone)]
pub struct Cache<B, Req, Res> {
    pub backend: Arc<B>,
    request_predicates: Arc<dyn Predicate<Subject = Req> + Send + Sync>,
    response_predicates: Arc<dyn Predicate<Subject = Res> + Send + Sync>,
    key_extractors: Arc<dyn Extractor<Subject = Req> + Send + Sync>,
}

impl<B, Req, Res> Cache<B, Req, Res> {
    pub fn new(backend: B) -> Cache<B, Req, Res> {
        Cache {
            backend: Arc::new(backend),
            request_predicates: Arc::new(NeutralPredicate::new()),
            response_predicates: Arc::new(NeutralResponsePredicate::new()),
            key_extractors: Arc::new(NeutralExtractor::new()),
        }
    }
}

impl<S, B, Req, Res> Layer<S> for Cache<B, Req, Res> {
    type Service = CacheService<S, B>;

    fn layer(&self, upstream: S) -> Self::Service {
        CacheService::new(upstream, Arc::clone(&self.backend))
    }
}

impl<Req, Res> Cache<DummyBackend, Req, Res> {
    pub fn builder() -> CacheBuilder<DummyBackend, Req, Res> {
        CacheBuilder::<DummyBackend>::default()
    }
}

pub struct CacheBuilder<B, Req, Res> {
    backend: Option<B>,
}

impl<B, Req, Res> CacheBuilder<B, Req, Res>
where
    B: CacheBackend,
{
    pub fn backend<NB: CacheBackend>(self, backend: NB) -> CacheBuilder<NB> {
        CacheBuilder {
            backend: Some(backend),
        }
    }

    pub fn build(self) -> Cache<B, Req, Res> {
        Cache {
            backend: Arc::new(self.backend.expect("Please add some cache backend")),
        }
    }
}

impl<B> Default for CacheBuilder<B> {
    fn default() -> Self {
        Self { backend: None }
    }
}
