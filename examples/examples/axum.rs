use axum::{extract::Path, routing::get, Json, Router};
use hitbox_tower::{
    configuration::extractor,
    configuration::predicate::{request, response},
    Method, StatusCode,
};

use hitbox_redis::RedisBackend;
use hitbox_tower::{Cache, EndpointConfig};

async fn handler_result(Path(_name): Path<String>) -> Result<String, String> {
    //dbg!("axum::handler_result");
    // Ok(format!("Hello, {name}"))
    Err("error".to_owned())
}

async fn handler() -> String {
    //dbg!("axum::handler");
    "root".to_string()
}

#[derive(serde::Serialize)]
struct Greet {
    name: String,
    answer: u32,
}

async fn handler_json() -> Json<Greet> {
    //dbg!("axum::handler");
    Json(Greet {
        name: "root".to_owned(),
        answer: 42,
    })
}

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .pretty()
        .with_env_filter("debug,hitbox=trace")
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let redis_backend = RedisBackend::new().unwrap();
    let inmemory_backend = hitbox_stretto::StrettoBackend::builder(10)
        .finalize()
        .unwrap();

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

    let health_config = EndpointConfig::builder()
        .request(request::path("/health").method(Method::GET))
        .disable()
        .build();

    let json_cache = Cache::builder()
        .backend(redis_backend)
        .config(json_config)
        .build();

    let health_check = Cache::builder()
        .backend(inmemory_backend) // FIX: it should work withod backend
        .config(health_config)
        .build();

    let app = Router::new()
        .route("/greet/:name/", get(handler_result))
        .route("/", get(handler))
        .route("/json/", get(handler_json))
        .route("/health", get(handler).layer(health_check))
        .layer(json_cache);

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
