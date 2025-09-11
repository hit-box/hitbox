use crate::app::app;
use hitbox_configuration::Endpoint;
use hitbox_moka::MokaBackend;
use hitbox_tower::Cache;
use std::fmt::Debug;
use std::str::FromStr;

use anyhow::Error;
use axum_test::{TestResponse, TestServer};
use cucumber::gherkin::Step;
use cucumber::World;
use hurl::http::{Body, RequestSpec};

#[derive(Debug, Default)]
pub struct State {
    pub response: Option<TestResponse>,
}

#[derive(Debug, World)]
pub struct HitboxWorld {
    pub config: Endpoint<axum::body::Body, axum::body::Body>,
    pub state: State,
    pub backend: MokaBackend,
}

impl Default for HitboxWorld {
    fn default() -> Self {
        Self {
            config: Default::default(),
            state: Default::default(),
            backend: MokaBackend::builder(100).build(),
        }
    }
}

impl HitboxWorld {
    pub async fn execute_request(&mut self, request_spec: &RequestSpec) -> Result<(), Error> {
        let cache = Cache::builder()
            .backend(self.backend.clone())
            .config(self.config.clone())
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
