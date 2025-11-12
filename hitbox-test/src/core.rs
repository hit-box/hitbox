use crate::app::app;
use crate::time::{MockTime, MockTimeProvider, clear_mock_time_provider};
use hitbox_configuration::Endpoint;
use hitbox_http::BufferedBody;
use hitbox_moka::MokaBackend;
use hitbox_tower::Cache;
use std::fmt::Debug;
use std::str::FromStr;

use anyhow::Error;
use axum_test::{TestResponse, TestServer};
use cucumber::World;
use cucumber::gherkin::Step;
use hurl::http::{Body, RequestSpec};

#[derive(Debug, Default)]
pub struct State {
    pub response: Option<TestResponse>,
}

#[derive(Debug, Clone, Default)]
pub struct TimeState {
    pub mock_time: Option<MockTime>,
    pub mock_provider: Option<MockTimeProvider>,
}

#[derive(Debug, World)]
pub struct HitboxWorld {
    pub config: Endpoint<BufferedBody<axum::body::Body>, BufferedBody<axum::body::Body>>,
    pub state: State,
    pub backend: MokaBackend,
    pub time_state: TimeState,
}

impl Default for HitboxWorld {
    fn default() -> Self {
        Self {
            config: Default::default(),
            state: Default::default(),
            backend: MokaBackend::builder(100).build(),
            time_state: Default::default(),
        }
    }
}

impl Drop for HitboxWorld {
    fn drop(&mut self) {
        // Clean up global mock time provider when scenario ends
        // This ensures each scenario has isolated time state
        if self.time_state.mock_time.is_some() {
            clear_mock_time_provider();
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
