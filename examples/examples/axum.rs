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
