use hitbox_stretto::builder::StrettoBackendBuilder;
use hitbox_tower::Cache;
use hyper::{Body, Server};
use std::{convert::Infallible, net::SocketAddr};

use http::{Request, Response};
use tower::make::Shared;

async fn handle(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("Hello, World!".into()))
}

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::fmt()
        .pretty()
        .with_env_filter("debug,hitbox=trace")
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let inmemory = StrettoBackendBuilder::new(12960, 1e6 as i64)
        .finalize()
        .unwrap();
    let service = tower::ServiceBuilder::new()
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(Cache::builder().backend(inmemory).build())
        .service_fn(handle);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    Server::bind(&addr)
        .serve(Shared::new(service))
        .await
        .expect("server error");
}
