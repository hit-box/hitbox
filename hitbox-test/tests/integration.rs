use std::str::FromStr;

use anyhow::{anyhow, Context, Error};
use axum::{extract::Path, routing::get, Router};
use axum_test::{TestResponse, TestServer};
use cucumber::{gherkin::Step, given, then, when, World};
use hitbox::policy::PolicyConfig;
use hitbox_test::Predicates;
use hitbox_tower::{
    configuration::{
        extractor,
        predicate::{request, response},
    },
    Cache, EndpointConfig,
};
use http::StatusCode;
use hurl::{
    http::{Body, RequestSpec},
    runner::{request::eval_request, VariableSet},
    util::path::ContextDir,
};
use hurl_core::{ast::Method, error::DisplaySourceError, parser::parse_hurl_file, text::Format};

fn main() {
    futures::executor::block_on(HitboxWorld::run("tests/features/basic.feature"));
}

async fn handler_result(
    Path(name): Path<String>,
    request: axum::extract::Request,
) -> Result<String, StatusCode> {
    dbg!("axum::handler_result");
    Ok(format!("Hello, {name}"))
}

#[derive(Debug, World, Default)]
pub struct HitboxWorld {
    predicates: Predicates,
    response: Option<TestResponse>,
    policy: PolicyConfig,
}

impl HitboxWorld {
    async fn execute_request(&mut self, request_spec: &RequestSpec) -> Result<(), Error> {
        let app = Router::new().route("/greet/{name}", get(handler_result));
        // .layer(json_cache);

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
        self.response = Some(response);
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

#[given(regex = r"hitbox with policy")]
fn hitbox(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    dbg!(step.docstring_content());
    let policy = step
        .docstring_content()
        .as_deref()
        .map(serde_yaml::from_str::<PolicyConfig>)
        .transpose()?
        .unwrap_or_default();
    world.policy = policy;
    Ok(())
}

#[when(expr = "execute request")]
async fn execute(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let hurl_request = step
        .docstring_content()
        .ok_or_else(|| anyhow!("request not provided"))?;
    let hurl_file = parse_hurl_file(&hurl_request).map_err(|err| {
        anyhow!(
            "hurl request parse error: {}",
            &err.message(&hurl_request.lines().collect::<Vec<_>>())
                .to_string(Format::Ansi)
        )
    })?;
    let variables = VariableSet::new();
    let request = &hurl_file
        .entries
        .get(0)
        .ok_or_else(|| anyhow!("request not found"))?
        .request;
    let request = eval_request(&request, &variables, &ContextDir::default())
        .map_err(|err| anyhow!("hurl request error {:?}", err))?;
    world.execute_request(&request).await?;
    dbg!(&world.response);
    Ok(())
}

#[given(expr = "request predicates")]
async fn request_predicates(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    Ok(())
}

#[given(expr = "key extractor {string}")]
async fn key_extractor(
    world: &mut HitboxWorld,
    step: &Step,
    extractor: String,
) -> Result<(), Error> {
    Ok(())
}

#[then(expr = "response status is {int}")]
async fn check_response_status(
    world: &mut HitboxWorld,
    step: &Step,
    status: u16,
) -> Result<(), Error> {
    Ok(())
}

#[then(expr = "cache has records")]
async fn check_cache_backend_state(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    Ok(())
}

// ====================================

#[given(expr = "request predicate {word}")]
fn request_predicate(world: &mut HitboxWorld, step: &Step, predicate: String) {
    world.predicates.request.push(predicate);
    dbg!(&world);
}

// #[when(expr = r#"I send a {word} request to {string}"#)]
// async fn set_method_and_path(world: &mut HitboxWorld, method: String, path: String) {
//     let method = Method::from_bytes(method.as_bytes()).unwrap();
//     world.request.method = Some(method);
//     world.request.path = Some(path);
// }

// #[when("I set headers:")]
// async fn set_headers(world: &mut HitboxWorld, step: &Step) {
//     if let Some(table) = &step.table {
//         for row in table.rows.iter() {
//             let key = row[0].clone();
//             let value = row[1].clone();
//             world.request.headers.push((key, value));
//         }
//     }
// }

// #[when(expr = r#"the request body is:"#)]
// async fn set_body(world: &mut HitboxWorld, step: &Step) {
//     if let Some(docstring) = &step.docstring {
//         world.request.body = Some(docstring.clone());
//     }
// }

// async fn execute_fsm(world: &mut HitboxWorld) {
//     dbg!("execute FSM");
//     let inmemory_backend = hitbox_moka::MokaBackend::builder(1024 * 1024).build();
//     let json_config = EndpointConfig::builder()
//         .request(
//             request::method(Method::GET)
//                 .query("cache", "true")
//                 .query("x-cache", "true")
//                 .path("/{path}*"),
//         )
//         .response(response::status_code(StatusCode::OK))
//         .cache_key(extractor::method().query("cache").path("/{path}*"))
//         .build();
//
//     let json_cache = Cache::builder()
//         .backend(inmemory_backend.clone())
//         .config(json_config)
//         .build();
//
//     let app = Router::new()
//         .route("/greet/{name}", get(handler_result))
//         .layer(json_cache);
//
//     let server = TestServer::new(app).unwrap();
//
//     let method = world
//         .request
//         .method
//         .as_ref()
//         .expect("Request method not set");
//     let path = world.request.path.as_ref().expect("Request path not set");
//     let mut request = match method {
//         &Method::GET => server.get(path),
//         &Method::POST => server.post(path),
//         _ => panic!("Unsupported method: {}", method),
//     };
//     for (key, value) in &world.request.headers {
//         request = request.add_header(key, value);
//     }
//     if let Some(body) = &world.request.body {
//         request = request.json(body);
//     }
//
//     let response = request.await;
//     world.response = Some(response);
// }

// #[cucumber::then(expr = "the response status should be {int}")]
// fn check_response_status(world: &mut HitboxWorld, expected: u16) {
//     let response = world.response.as_ref().expect("No response found");
//     assert_eq!(
//         response.status_code().as_u16(),
//         expected,
//         "Unexpected response status"
//     );
// }
