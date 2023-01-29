use axum::{routing::get, Router};
use hitbox_actix::RedisBackend;
use hitbox_tower::Cache;
use tower::ServiceBuilder;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(
            ServiceBuilder::new().layer(
                Cache::builder()
                    .backend(RedisBackend::new().unwrap())
                    .build(),
            ),
        );

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
