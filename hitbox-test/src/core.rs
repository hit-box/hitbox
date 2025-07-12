use hitbox::config::CacheConfig;
use hitbox::{Extractor, Predicate};
use hitbox_moka::MokaBackend;
use hitbox_tower::Cache;
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Error};
use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
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
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Handler {
    pub path: String,
    pub method: String,
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl Default for Handler {
    fn default() -> Self {
        Self {
            path: "/greet/{name}".to_owned(),
            method: "GET".to_owned(),
            status_code: 200,
            headers: HashMap::new(),
            body: None,
        }
    }
}

#[derive(Clone)]
pub struct Settings {
    pub policy: PolicyConfig,
    pub handler: Handler,
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
            handler: Handler::default(),
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
        let handler_config = self.settings.handler.clone();
        let headers_clone = handler_config.headers.clone();
        let body_clone = handler_config.body.clone();
        let status_code = handler_config.status_code;

        let handler = move || async move {
            let mut headers = http::header::HeaderMap::new();
            for (name, value) in headers_clone {
                if let (Ok(header_name), Ok(header_value)) = (
                    http::header::HeaderName::from_str(&name),
                    http::header::HeaderValue::from_str(&value),
                ) {
                    headers.insert(header_name, header_value);
                }
            }

            let body = body_clone.unwrap_or_else(|| String::new());
            dbg!(&body);

            (StatusCode::from_u16(status_code).unwrap(), headers, body)
        };
        let cache = Cache::builder()
            .backend(self.backend.clone())
            .config(self.settings.clone())
            .build();
        let method_router = match handler_config.method.to_uppercase().as_str() {
            "GET" => get(handler),
            "POST" => post(handler),
            "PUT" => put(handler),
            "DELETE" => delete(handler),
            "PATCH" => patch(handler),
            _ => get(handler), // Default to GET for unsupported methods
        };

        let router = Router::new()
            .route(&handler_config.path, method_router)
            .layer(cache);

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
