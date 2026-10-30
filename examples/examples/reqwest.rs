//! Reqwest Client Caching Example
//!
//! Demonstrates client-side HTTP caching with the reqwest HTTP client.
//!
//! Features shown:
//!   - Integration with reqwest-middleware
//!   - Moka in-memory backend for client-side caching
//!   - Request predicate: only cache GET requests
//!   - Cache key extraction from method and path
//!   - X-Cache-Status header for cache status (hit/miss/stale)
//!
//! Run:
//!   cargo run -p hitbox-examples --example reqwest
//!
//! What it does:
//!   - Fetches GitHub API data for the hitbox repository
//!   - First request: cache miss (fetches from GitHub)
//!   - Second request: cache hit (returns cached response)
//!
//! Try it:
//!   Run the example and observe the logs showing cache status transitions.

use std::time::Duration;

use hitbox::Config;
use hitbox::policy::PolicyConfig;
use hitbox_http::{
    extractors::{MethodConfig, MethodExtractor, PathExtractor},
    predicates::request::MethodPredicate,
    request, response,
};
use hitbox_moka::MokaBackend;
use hitbox_reqwest::CacheMiddleware;
use reqwest::Client;
use reqwest_middleware::ClientBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debugging
    tracing_subscriber::fmt()
        .with_env_filter("hitbox=debug")
        .init();

    // Create a Moka in-memory backend
    let backend = MokaBackend::builder().max_entries(1000).build();

    // Configure cache endpoint using builder pattern
    // - Only cache GET requests
    // - Cache key includes: method + path
    // - TTL: 60 seconds
    let config = Config::builder()
        .request_predicate(request::predicate().method(http::Method::GET))
        .response_predicate(response::predicate())
        .extractor(
            request::extractor()
                .method(MethodConfig::new())
                .path("/{path}*"),
        )
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    // Create the cache middleware using builder pattern
    let cache_middleware = CacheMiddleware::builder()
        .backend(backend)
        .config(config)
        .build();

    // Build the client with middleware
    let client = ClientBuilder::new(Client::new())
        .with(cache_middleware)
        .build();

    // GitHub API for hitbox repo (requires User-Agent header)
    let url = "https://api.github.com/repos/hit-box/hitbox";

    tracing::info!("First request (cache miss)");
    let response = client
        .get(url)
        .header("User-Agent", "hitbox-example/1.0")
        .send()
        .await?;
    tracing::info!(
        status = %response.status(),
        cache_status = ?response.headers().get("X-Cache-Status"),
        "Response received"
    );
    let body = response.text().await?;
    tracing::info!(body_length = body.len(), "Body received");

    tracing::info!("Second request (should be cache hit)");
    let response = client
        .get(url)
        .header("User-Agent", "hitbox-example/1.0")
        .send()
        .await?;
    tracing::info!(
        status = %response.status(),
        cache_status = ?response.headers().get("X-Cache-Status"),
        "Response received"
    );
    let body = response.text().await?;
    tracing::info!(body_length = body.len(), "Body received");

    Ok(())
}
