use hitbox_redis::RedisBackend;
use hitbox_stretto::StrettoBackend;
use hitbox_tower::Cache;
use hyper::{Body, Server};
use std::net::SocketAddr;

use http::{Request, Response};
use tower::make::Shared;

async fn handle(_: Request<Body>) -> Result<Response<Body>, String> {
    Ok(Response::new("Hello, World!".into()))
    // Err("handler error".to_owned())
}

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .pretty()
        .with_env_filter("debug,hitbox=trace")
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let inmemory = StrettoBackend::builder(10_000_000).finalize().unwrap();
    let redis = RedisBackend::builder().build().unwrap();

    let service = tower::ServiceBuilder::new()
        .layer(Cache::builder().backend(inmemory).build())
        .layer(Cache::builder().backend(redis).build())
        .service_fn(handle);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    Server::bind(&addr)
        .serve(Shared::new(service))
        .await
        .expect("server error");
}
