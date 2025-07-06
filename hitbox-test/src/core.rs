use hitbox::config::{CacheConfig, RequestExtractor, RequestPredicate, ResponsePredicate};
use hitbox_tower::Cache;
use std::fmt::{self, Debug};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Error};
use axum::{extract::Path, routing::get, Router};
use axum_test::{TestResponse, TestServer};
use cucumber::gherkin::Step;
use cucumber::World;
use hitbox::policy::PolicyConfig;
use hitbox_http::{
    extractors::NeutralExtractor,
    predicates::{NeutralRequestPredicate, NeutralResponsePredicate},
};
use hitbox_http::{CacheableHttpRequest, CacheableHttpResponse};
use http::StatusCode;
use hurl::http::{Body, RequestSpec};

pub struct Settings {
    pub policy: PolicyConfig,
    pub extractors: Arc<dyn hitbox::Extractor<Subject = CacheableHttpRequest<axum::body::Body>> + Send + Sync>,
    pub request_predicates:
        Box<dyn hitbox::Predicate<Subject = CacheableHttpRequest<axum::body::Body>>>,
    pub response_predicates:
        Box<dyn hitbox::Predicate<Subject = CacheableHttpResponse<axum::body::Body>>>,
}

impl CacheConfig<axum::body::Body, axum::body::Body> for Settings
{
    type RequestBody = CacheableHttpRequest<axum::body::Body>;
    type ResponseBody = CacheableHttpResponse<axum::body::Body>;

    fn request_predicates(&self) -> RequestPredicate<Self::RequestBody> {
        self.request_predicates
    }

    fn response_predicates(&self) -> ResponsePredicate<Self::ResponseBody> {
        self.response_predicates
    }

    fn extractors(&self) -> RequestExtractor<Self::RequestBody> {
        self.extractors
    }

    fn policy(&self) -> PolicyConfig {
        PolicyConfig::default()
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
            extractors: Box::new(NeutralExtractor::new()),
            request_predicates: Box::new(NeutralRequestPredicate::new()),
            response_predicates: Box::new(NeutralResponsePredicate::new()),
        }
    }
}

#[derive(Debug, Default)]
pub struct State {
    pub response: Option<TestResponse>,
}

#[derive(Debug, World, Default)]
pub struct HitboxWorld {
    pub settings: Settings,
    pub state: State,
}

async fn handler_result(
    Path(name): Path<String>,
    _request: axum::extract::Request,
) -> Result<String, StatusCode> {
    dbg!("axum::handler_result");
    Ok(format!("Hello, {name}"))
}

impl HitboxWorld {
    pub async fn execute_request(&mut self, request_spec: &RequestSpec) -> Result<(), Error> {
        let inmemory = hitbox_moka::MokaBackend::builder(1).build();
        let cache = Cache::builder().backend(inmemory).build();
        let app = Router::new()
            .route("/{*path}", get(handler_result))
            .layer(cache);

        let server = TestServer::new(app)?;
        let path = request_spec.url.path();
        let mut request = server.method(
            http::Method::from_str(request_spec.method.0.to_string().as_str())?,
            path.as_str(),
        );
        for header in &request_spec.headers {
            request = request.add_header(&header.name, &header.value);
        }
        let request = match &request_spec.body {
            Body::Text(body) => Ok(request.json(body)),
            _ => Err(anyhow!("unsupported body type")),
        }?;

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
