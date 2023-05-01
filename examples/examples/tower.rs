use hitbox_redis::RedisBackend;
use hitbox_tower::Cache;
use hyper::{Body, Server};
use lazy_static::lazy_static;
use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use http::{Request, Response};
use tower::{make::Shared, ServiceBuilder};

lazy_static! {
    static ref BACKEND: Arc<RedisBackend> = Arc::new(RedisBackend::new().unwrap());
}

async fn handle(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("Hello, World!".into()))
}

#[tokio::main]
async fn main() {
    let backend = RedisBackend::new().unwrap();

    let service = tower::ServiceBuilder::new()
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(
            ServiceBuilder::new().service(
                Cache::builder()
                    // .backend(&BACKEND)
                    .backend(backend)
                    .build(),
            ),
        )
        .service_fn(handle);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let server = Server::bind(&addr).serve(Shared::new(service));
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
