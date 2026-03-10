use crate::app::app;
use crate::handler_state::HandlerState;
use crate::mock_backend::MockBackend;
use hitbox::Config;
use hitbox::concurrency::BroadcastConcurrencyManager;
use hitbox::offload::OffloadManager;
use hitbox::policy::PolicyConfig;
use hitbox_http::CacheableHttpRequest;
use hitbox_http::CacheableHttpResponse;
use hitbox_http::extractors::NeutralExtractor;
use hitbox_http::predicates::{NeutralRequestPredicate, NeutralResponsePredicate};
use hitbox_tower::Cache;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Error;
use axum_test::{TestResponse, TestServer};
use cucumber::World;
use cucumber::gherkin::Step;
use hitbox::{Extractor, Predicate};
use hurl::http::{Body, RequestSpec};

#[derive(Debug, Default)]
pub struct State {
    pub response: Option<TestResponse>,
    pub responses: Vec<TestResponse>,
}

pub type BoxRequestPredicate = Box<
    dyn Predicate<Subject = CacheableHttpRequest<axum::body::Body>, Context = ()> + Send + Sync,
>;
pub type BoxResponsePredicate = Box<
    dyn Predicate<Subject = CacheableHttpResponse<axum::body::Body>, Context = ()> + Send + Sync,
>;
pub type BoxExtractor = Box<
    dyn Extractor<Subject = CacheableHttpRequest<axum::body::Body>, Context = ()> + Send + Sync,
>;

/// Holds cache configuration components that can be modified by test steps.
pub struct TestConfig {
    pub request_predicate: Arc<BoxRequestPredicate>,
    pub response_predicate: Arc<BoxResponsePredicate>,
    pub extractor: Arc<BoxExtractor>,
    pub policy: PolicyConfig,
}

impl std::fmt::Debug for TestConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestConfig")
            .field("request_predicate", &"...")
            .field("response_predicate", &"...")
            .field("extractor", &"...")
            .field("policy", &self.policy)
            .finish()
    }
}

impl Clone for TestConfig {
    fn clone(&self) -> Self {
        Self {
            request_predicate: Arc::clone(&self.request_predicate),
            response_predicate: Arc::clone(&self.response_predicate),
            extractor: Arc::clone(&self.extractor),
            policy: self.policy.clone(),
        }
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        let request_predicate: BoxRequestPredicate =
            Box::new(NeutralRequestPredicate::<axum::body::Body>::new());
        let response_predicate: BoxResponsePredicate =
            Box::new(NeutralResponsePredicate::<axum::body::Body>::new());
        let extractor: BoxExtractor = Box::new(NeutralExtractor::<axum::body::Body>::new());
        Self {
            request_predicate: Arc::new(request_predicate),
            response_predicate: Arc::new(response_predicate),
            extractor: Arc::new(extractor),
            policy: PolicyConfig::default(),
        }
    }
}

impl TestConfig {
    pub fn build(
        &self,
    ) -> Config<Arc<BoxRequestPredicate>, Arc<BoxResponsePredicate>, Arc<BoxExtractor>> {
        Config::builder()
            .request_predicate(Arc::clone(&self.request_predicate))
            .response_predicate(Arc::clone(&self.response_predicate))
            .extractor(Arc::clone(&self.extractor))
            .policy(self.policy.clone())
            .build()
    }
}

#[derive(Debug, World)]
pub struct HitboxWorld {
    pub config: TestConfig,
    pub state: State,
    pub backend: MockBackend,
    #[world(default)]
    pub offload_manager: Option<OffloadManager>,
    pub handler_state: HandlerState,
}

impl Default for HitboxWorld {
    fn default() -> Self {
        Self {
            config: Default::default(),
            state: Default::default(),
            backend: MockBackend::new(),
            offload_manager: None,
            handler_state: HandlerState::new(),
        }
    }
}

impl HitboxWorld {
    pub async fn execute_request(&mut self, request_spec: &RequestSpec) -> Result<(), Error> {
        let concurrency_manager: BroadcastConcurrencyManager<_> =
            BroadcastConcurrencyManager::new();
        let config = self.config.build();

        let server = if let Some(manager) = &self.offload_manager {
            let cache = Cache::builder()
                .backend(self.backend.clone())
                .config(config)
                .concurrency_manager(concurrency_manager)
                .offload(manager.clone())
                .build();
            let router = app(self.handler_state.clone()).layer(cache);
            TestServer::new(router)?
        } else {
            let cache = Cache::builder()
                .backend(self.backend.clone())
                .config(config)
                .concurrency_manager(concurrency_manager)
                .build();
            let router = app(self.handler_state.clone()).layer(cache);
            TestServer::new(router)?
        };
        let path = request_spec.url.path();
        let mut request = server.method(
            http::Method::from_str(request_spec.method.0.to_string().as_str())?,
            path.as_ref(),
        );
        for header in &request_spec.headers {
            request = request.add_header(&header.name, &header.value);
        }
        // Add query params from URL
        for param in request_spec.url.query_params() {
            request = request.add_query_param(&param.name, &param.value);
        }
        // Add query params from [Query] section
        for param in &request_spec.querystring {
            request = request.add_query_param(&param.name, &param.value);
        }

        // Set request body based on content type
        // Use .text() and .bytes() methods which don't modify headers,
        // unlike .json() which automatically sets Content-Type: application/json
        let request = match &request_spec.body {
            Body::Text(body) if !body.is_empty() => request.text(body),
            Body::File(body, _name) if !body.is_empty() => request.bytes(body.clone().into()),
            Body::Binary(bin) if !bin.is_empty() => request.bytes(bin.clone().into()),
            _ => request, // No body
        };

        let response = request.await;
        self.state.response = Some(response);

        Ok(())
    }
}

pub trait StepExt {
    fn docstring_content(&self) -> Option<String>;
}

impl StepExt for Step {
    fn docstring_content(&self) -> Option<String> {
        self.docstring()
            .map(|docstring| docstring.lines().skip(1).collect::<Vec<_>>().join("\n"))
    }
}
