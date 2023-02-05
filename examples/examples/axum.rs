use std::sync::Arc;

use axum::{routing::get, Router};
use hitbox_actix::RedisBackend;
use hitbox_tower::Cache;
use lazy_static::lazy_static;
use tower::ServiceBuilder;

lazy_static! {
    static ref BACKEND: Arc<RedisBackend> = Arc::new(RedisBackend::new().unwrap());
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    let backend = Arc::new(RedisBackend::new().unwrap());
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(
            ServiceBuilder::new().layer(
                Cache::builder()
                    .backend(&BACKEND)
                    .build(),
            ),
        );

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
