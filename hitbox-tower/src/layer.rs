use crate::{
    config::EndpointConfig, request_extractor::ExtractorBuilder,
    request_predicate::RequestPredicateBuilder, response_predicate::ResponsePredicateBuilder,
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
    pub endpoint_config: Arc<EndpointConfig>,
    pub policy: Arc<PolicyConfig>,
}

impl<B> Cache<B> {
    pub fn new(backend: B) -> Cache<B> {
        Cache {
            backend: Arc::new(backend),
            endpoint_config: Arc::new(Default::default()),
            policy: Arc::new(Default::default()),
        }
    }
}

impl<S, B> Layer<S> for Cache<B> {
    type Service = CacheService<S, B>;

    fn layer(&self, upstream: S) -> Self::Service {
        CacheService::new(
            upstream,
            Arc::clone(&self.backend),
            Arc::clone(&self.endpoint_config),
            Arc::clone(&self.policy),
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
    endpoint_config: EndpointConfig,
    policy: PolicyConfig,
}

impl<B> CacheBuilder<B>
where
    B: CacheBackend,
{
    pub fn backend<NB: CacheBackend>(self, backend: NB) -> CacheBuilder<NB> {
        CacheBuilder {
            backend: Some(backend),
            endpoint_config: self.endpoint_config,
            policy: self.policy,
        }
    }

    pub fn enable(self, policy: EnabledCacheConfig) -> Self {
        CacheBuilder {
            backend: self.backend,
            endpoint_config: self.endpoint_config,
            policy: PolicyConfig::Enabled(policy),
        }
    }

    pub fn disable(self) -> Self {
        CacheBuilder {
            backend: self.backend,
            endpoint_config: self.endpoint_config,
            policy: PolicyConfig::Disabled,
        }
    }

    pub fn request(self, predicates: RequestPredicateBuilder) -> Self {
        let endpoint_config = EndpointConfig {
            request_predicates: predicates.build(),
            response_predicates: self.endpoint_config.response_predicates,
            extractors: self.endpoint_config.extractors,
        };
        CacheBuilder {
            backend: self.backend,
            endpoint_config,
            policy: self.policy,
        }
    }

    pub fn response(self, predicates: ResponsePredicateBuilder) -> Self {
        let endpoint_config = EndpointConfig {
            request_predicates: self.endpoint_config.request_predicates,
            response_predicates: predicates.build(),
            extractors: self.endpoint_config.extractors,
        };
        CacheBuilder {
            backend: self.backend,
            endpoint_config,
            policy: self.policy,
        }
    }

    pub fn cache_key(self, extractors: ExtractorBuilder) -> Self {
        let endpoint_config = EndpointConfig {
            request_predicates: self.endpoint_config.request_predicates,
            response_predicates: self.endpoint_config.response_predicates,
            extractors: extractors.build(),
        };
        CacheBuilder {
            backend: self.backend,
            endpoint_config,
            policy: self.policy,
        }
    }

    pub fn build(self) -> Cache<B> {
        Cache {
            backend: Arc::new(self.backend.expect("Please add some cache backend")),
            endpoint_config: Arc::new(self.endpoint_config),
            policy: Arc::new(self.policy),
        }
    }
}

impl<B> Default for CacheBuilder<B> {
    fn default() -> Self {
        Self {
            backend: None,
            endpoint_config: Default::default(),
            policy: Default::default(),
        }
    }
}
