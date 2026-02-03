//! Composition Backend Example
//!
//! Demonstrates a 3-tier cache hierarchy using composable backends.
//!
//! Cache layers:
//! - L1: Moka (in-memory) - fastest, limited capacity
//! - L2: FeOxDB (file-based) - persistent, local storage
//! - L3: Redis (distributed) - shared across instances
//!
//! Features shown:
//! - Composing multiple backends with fluent API
//! - Refill policies (Always) to populate upper tiers on cache miss
//! - OffloadManager for background operations
//! - Labels for observability and debugging
//!
//! Prerequisites:
//!   Redis server running on localhost:6379
//!
//! Run:
//!   cargo run -p hitbox-examples --example composition
//!
//! Endpoints:
//!   - http://localhost:3000/ - Hello World (cached, TTL: 60s)
//!
//! Try it:
//!   curl -v http://localhost:3000/   # First: miss on all tiers, populates cache
//!   curl -v http://localhost:3000/   # Second: hit on L1 (Moka)

use std::time::Duration;

use axum::{Router, routing::get};
use hitbox::Config;
use hitbox::offload::OffloadManager;
use hitbox::policy::PolicyConfig;
use hitbox_backend::composition::{Compose, policy::RefillPolicy};
use hitbox_feoxdb::FeOxDbBackend;
use hitbox_http::extractors::{self, MethodConfig, MethodExtractor, PathExtractor};
use hitbox_http::predicates::{NeutralRequestPredicate, NeutralResponsePredicate};
use hitbox_moka::MokaBackend;
use hitbox_redis::{ConnectionMode, RedisBackend};
use hitbox_tower::Cache;
use tempfile::TempDir;

async fn hello() -> &'static str {
    tracing::info!("Handler called - fetching from upstream");
    "Hello, World!"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info,hitbox=debug")
        .init();

    // L1: Moka (in-memory)
    let moka = MokaBackend::builder()
        .max_entries(1024 * 1024)
        .label("moka")
        .build();

    // L2: FeOxDB (file-based)
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let feoxdb = FeOxDbBackend::builder()
        .path(temp_dir.path())
        .build()
        .expect("Failed to open FeOxDB");

    // L3: Redis (distributed)
    let redis = RedisBackend::builder()
        .connection(ConnectionMode::single("redis://127.0.0.1/"))
        .label("redis")
        .build()
        .expect("Redis connection failed. Ensure Redis is running on localhost:6379");

    // Compose: Moka → FeOxDB → Redis
    let offload = OffloadManager::with_defaults();

    let local = moka
        .compose(feoxdb, offload.clone())
        .label("local")
        .refill(RefillPolicy::Always);

    let composed = local
        .compose(redis, offload)
        .label("cache")
        .refill(RefillPolicy::Always);

    let config = Config::builder()
        .request_predicate(NeutralRequestPredicate::new())
        .response_predicate(NeutralResponsePredicate::new())
        .extractor(
            extractors::extractor()
                .method(MethodConfig::new())
                .path("/"),
        )
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    let cache = Cache::builder().backend(composed).config(config).build();

    let app = Router::new().route("/", get(hello).layer(cache));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");
    tracing::info!("Listening on http://localhost:3000");
    axum::serve(listener, app).await.expect("Server error");
}
