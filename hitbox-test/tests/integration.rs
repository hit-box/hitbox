use axum::{extract::Path, routing::get, Router};
use axum_test::TestServer;
use bytes::Bytes;
use cucumber::{gherkin::Step, given, when, World};
use hitbox::fsm::CacheFuture;
use hitbox_test::Predicates;
use hitbox_tower::{
    configuration::{
        extractor,
        predicate::{request, response},
    },
    Cache, EndpointConfig,
};
use http::{Method, Request, StatusCode};

// This runs before everything else, so you can setup things here.
fn main() {
    // You may choose any executor you like (`tokio`, `async-std`, etc.).
    // You may even have an `async` main, it doesn't matter. The point is that
    // Cucumber is composable. :)
    futures::executor::block_on(HitboxWorld::run("tests/features/basic.feature"));
}

async fn handler_result(Path(name): Path<String>) -> Result<String, StatusCode> {
    dbg!("axum::handler_result");
    Ok(format!("Hello, {name}"))
    // Err(StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Debug, World)]
pub struct HitboxWorld {
    predicates: Predicates,
    request: Request<Bytes>,
}

impl Default for HitboxWorld {
    fn default() -> Self {
        Self {
            predicates: Default::default(),
            request: Request::new(Bytes::from_static(b"")),
        }
    }
}

#[given(regex = r"^hitbox with\s+(policy (.*))$")]
fn hitbox(_world: &mut HitboxWorld, step: &Step, policy: String) {
    dbg!(&step.docstring);
    dbg!(policy);
}

#[given(expr = "request predicate {word}")]
fn request_predicate(world: &mut HitboxWorld, step: &Step, predicate: String) {
    world.predicates.request.push(predicate);
    dbg!(&world);
}

#[when("execute request")]
async fn execute_fsm(world: &mut HitboxWorld) {
    dbg!("execute FSM");
    let inmemory_backend = hitbox_moka::MokaBackend::builder(1024 * 1024).build();
    let json_config = EndpointConfig::builder()
        .request(
            request::method(Method::GET)
                .query("cache", "true")
                .query("x-cache", "true")
                .path("/{path}*"),
        )
        .response(response::status_code(StatusCode::OK))
        .cache_key(extractor::method().query("cache").path("/{path}*"))
        .build();

    let json_cache = Cache::builder()
        .backend(inmemory_backend.clone())
        .config(json_config)
        .build();

    let app = Router::new()
        .route("/greet/{name}", get(handler_result))
        .layer(json_cache);

    // Run the application for testing.
    let server = TestServer::new(app).unwrap();

    // Get the request.
    let response = server.get("/greet/test").await;

    dbg!(&response);
    // assert_eq!(response.text(), "pong!");
}
