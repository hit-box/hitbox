use std::net::SocketAddr;

use axum::{
    handler::{get, Handler},
    Router,
};
use axum::response::Html;

use hitbox_axum::CacheLayer;

#[tokio::main]
async fn main() {
    let cache_layer = CacheLayer::build()
        .ttl(30)
        .stale_ttl(30)
        .version(1)
        .key_prefix("ferris")
        .by_method()
        .by_path()
        .path_parser(| path: String | -> String { path.trim_start_matches("/v1").to_string() })
        .by_header("X-Request")
        .by_header("X-Location")
        .by_query()
        .finish();
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
