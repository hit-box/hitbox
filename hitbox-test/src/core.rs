use crate::app::app;
use hitbox::config::CacheConfig;
use hitbox::{Extractor, Predicate};
use hitbox_moka::MokaBackend;
use hitbox_tower::Cache;
use std::fmt::{self, Debug};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Error;
use axum_test::{TestResponse, TestServer};
use cucumber::gherkin::Step;
use cucumber::World;
use hitbox::policy::PolicyConfig;
use hitbox_http::{
    extractors::NeutralExtractor,
    predicates::{NeutralRequestPredicate, NeutralResponsePredicate},
};
use hitbox_http::{CacheableHttpRequest, CacheableHttpResponse};
use hurl::http::{Body, RequestSpec};

#[derive(Clone)]
pub struct Settings {
    pub policy: PolicyConfig,
    pub extractors:
        Arc<dyn hitbox::Extractor<Subject = CacheableHttpRequest<axum::body::Body>> + Send + Sync>,
    pub request_predicates:
        Arc<dyn hitbox::Predicate<Subject = CacheableHttpRequest<axum::body::Body>> + Send + Sync>,
    pub response_predicates:
        Arc<dyn hitbox::Predicate<Subject = CacheableHttpResponse<axum::body::Body>> + Send + Sync>,
}

impl CacheConfig<CacheableHttpRequest<axum::body::Body>, CacheableHttpResponse<axum::body::Body>>
    for Settings
{
    type RequestBody = CacheableHttpRequest<axum::body::Body>;
    type ResponseBody = CacheableHttpResponse<axum::body::Body>;

    fn request_predicates(
        &self,
    ) -> impl Predicate<Subject = Self::RequestBody> + Send + Sync + 'static {
        self.request_predicates.clone()
    }

    fn response_predicates(
        &self,
    ) -> impl Predicate<Subject = Self::ResponseBody> + Send + Sync + 'static {
        self.response_predicates.clone()
    }

    fn extractors(&self) -> impl Extractor<Subject = Self::RequestBody> + Send + Sync + 'static {
        self.extractors.clone()
    }

    fn policy(&self) -> PolicyConfig {
        self.policy.clone()
    }
}

impl Debug for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Settings")
            .field("policy", &self.policy)
            .field("extractors", &self.extractors)
            .field("request_predicates", &self.request_predicates)
            .field("response_predicates", &self.response_predicates)
            .finish()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            policy: PolicyConfig::default(),
            extractors: Arc::new(NeutralExtractor::new()),
            request_predicates: Arc::new(NeutralRequestPredicate::new()),
            response_predicates: Arc::new(NeutralResponsePredicate::new()),
        }
    }
}

#[derive(Debug, Default)]
pub struct State {
    pub response: Option<TestResponse>,
}

#[derive(Debug, World)]
pub struct HitboxWorld {
    pub settings: Settings,
    pub state: State,
    pub backend: MokaBackend,
}

impl Default for HitboxWorld {
    fn default() -> Self {
        Self {
            settings: Default::default(),
            state: Default::default(),
            backend: MokaBackend::builder(100).build(),
        }
    }
}

impl HitboxWorld {
    pub async fn execute_request(&mut self, request_spec: &RequestSpec) -> Result<(), Error> {
        let cache = Cache::builder()
            .backend(self.backend.clone())
            .config(self.settings.clone())
            .build();

        let router = app().layer(cache);

        let server = TestServer::new(router)?;
        let path = request_spec.url.path();
        let mut request = server.method(
            http::Method::from_str(request_spec.method.0.to_string().as_str())?,
            path.as_str(),
        );
        for header in &request_spec.headers {
            request = request.add_header(&header.name, &header.value);
        }
        for param in &request_spec.querystring {
            request = request.add_query_param(&param.name, &param.value);
        }
        let request = match &request_spec.body {
            Body::Text(body) => request.json(body),
            Body::File(_body, _name) => request.json("{}"),
            Body::Binary(_bin) => request.json("{}"),
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
