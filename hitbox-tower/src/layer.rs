use crate::configuration::{
    EndpointConfig, ExtractorBuilder, RequestPredicateBuilder, ResponsePredicateBuilder,
};
use std::sync::Arc;

use hitbox::{
    backend::CacheBackend,
    policy::{EnabledCacheConfig, PolicyConfig},
};
use hitbox_stretto::StrettoBackend;
use tower::Layer;

use crate::service::CacheService;

#[derive(Clone)]
pub struct Cache<B> {
    pub backend: Arc<B>,
    pub configuration: Arc<EndpointConfig>,
}

impl<B> Cache<B> {
    pub fn new(backend: B) -> Cache<B> {
        Cache {
            backend: Arc::new(backend),
            configuration: Arc::new(Default::default()),
        }
    }
}

impl<S, B> Layer<S> for Cache<B> {
    type Service = CacheService<S, B, EndpointConfig>;

    fn layer(&self, upstream: S) -> Self::Service {
        CacheService::new(
            upstream,
            Arc::clone(&self.backend),
            Arc::clone(&self.configuration),
        )
    }
}

impl Cache<StrettoBackend> {
    pub fn builder() -> CacheBuilder<StrettoBackend> {
        CacheBuilder::default()
    }
}

pub struct CacheBuilder<B> {
    backend: Option<B>,
    configuration: EndpointConfig,
}

impl<B> CacheBuilder<B>
where
    B: CacheBackend,
{
    pub fn backend<NB: CacheBackend>(self, backend: NB) -> CacheBuilder<NB> {
        CacheBuilder {
            backend: Some(backend),
            configuration: self.configuration,
        }
    }

    pub fn enable(self, policy: EnabledCacheConfig) -> Self {
        let configuration = EndpointConfig {
            request_predicates: self.configuration.request_predicates,
            response_predicates: self.configuration.response_predicates,
            extractors: self.configuration.extractors,
            policy: PolicyConfig::Enabled(policy),
        };
        CacheBuilder {
            backend: self.backend,
            configuration,
        }
    }

    pub fn disable(self) -> Self {
        CacheBuilder {
            backend: self.backend,
            configuration: self.configuration,
        }
    }

    pub fn request(self, predicates: RequestPredicateBuilder) -> Self {
        let configuration = EndpointConfig {
            request_predicates: predicates.build(),
            response_predicates: self.configuration.response_predicates,
            extractors: self.configuration.extractors,
            policy: self.configuration.policy,
        };
        CacheBuilder {
            backend: self.backend,
            configuration,
        }
    }

    pub fn response(self, predicates: ResponsePredicateBuilder) -> Self {
        let configuration = EndpointConfig {
            request_predicates: self.configuration.request_predicates,
            response_predicates: predicates.build(),
            extractors: self.configuration.extractors,
            policy: self.configuration.policy,
        };
        CacheBuilder {
            backend: self.backend,
            configuration,
        }
    }

    pub fn cache_key(self, extractors: ExtractorBuilder) -> Self {
        let configuration = EndpointConfig {
            request_predicates: self.configuration.request_predicates,
            response_predicates: self.configuration.response_predicates,
            extractors: extractors.build(),
            policy: self.configuration.policy,
        };
        CacheBuilder {
            backend: self.backend,
            configuration,
        }
    }

    pub fn build(self) -> Cache<B> {
        Cache {
            backend: Arc::new(self.backend.expect("Please add some cache backend")),
            configuration: Arc::new(self.configuration),
        }
    }
}

impl<B> Default for CacheBuilder<B> {
    fn default() -> Self {
        Self {
            backend: None,
            configuration: Default::default(),
        }
    }
}
