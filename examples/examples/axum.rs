use axum::{routing::get, Router};
use hitbox_tower::state::Cache;
use hitbox_redis::actor::RedisBackend;
use tower::ServiceBuilder;

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    let cache = Cache::<RedisBackend>::new();
    let layer = ServiceBuilder::new().layer(cache);

    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(layer);

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
