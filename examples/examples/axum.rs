use axum::{extract::Path, routing::get, Json, Router};

use hitbox_redis::RedisBackend;
use hitbox_stretto::StrettoBackend;
use hitbox_tower::Cache;
use http::StatusCode;
use tower::ServiceBuilder;

async fn handler_result(Path(name): Path<String>) -> Result<String, StatusCode> {
    dbg!("axum::handler_result");
    Ok(format!("Hello, {name}"))
}

async fn handler() -> String {
    dbg!("axum::handler");
    format!("root")
}

#[derive(serde::Serialize)]
struct Greet {
    name: String,
    answer: u32,
}

async fn handler_json() -> Json<Greet> {
    dbg!("axum::handler");
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

    let backend = RedisBackend::new().unwrap();
    let inmemory = StrettoBackend::builder(2 ^ 16)
        .finalize()
        .unwrap();
    // build our application with a single route
    let app = Router::new()
        .route("/greet/:name/", get(handler_result))
        .route("/", get(handler))
        .route("/json/", get(handler_json))
        .layer(
            ServiceBuilder::new()
                .layer(Cache::builder().backend(inmemory).build())
                .layer(Cache::builder().backend(backend).build()),
        );

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}