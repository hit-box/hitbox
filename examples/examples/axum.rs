use std::net::SocketAddr;

use axum::{
    handler::{get, Handler},
    Router,
};
use axum::response::Html;

use hitbox_axum::CacheLayer;

#[tokio::main]
async fn main() {
    let cache_layer = CacheLayer::new();
    // let cache_layer = CacheLayer::build()
    //     .with_ttl(30)
    //     .with_stale(30)
    //     .with_version(1)
    //     .with_cache_key_prefix("тыдыщь")
    //     .by_path()
    //     .by_path_extended(parse)
    //     .by_header("X-Request")
    //     .by_header("X-Location")
    //     .by_query()
    //     .by_body()
    //     .finish();
    let app = Router::new().route("/users/:user_id/", get(handler.layer(cache_layer)));

    let addr: SocketAddr = ([127, 0, 0, 1], 3000).into();
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> Html<&'static str> {
    Html::from("<h1>Header</h1>")
}
