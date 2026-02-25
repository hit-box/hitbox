//! Hyper Client with Hitbox Caching Example
//!
//! Demonstrates using the hitbox-tower Cache layer with hyper client
//! for client-side HTTP response caching.
//!
//! The same Cache middleware works for both:
//! - Server-side (wrapping handlers in axum/hyper server)
//! - Client-side (wrapping hyper client for outgoing requests)
//!
//! Run:
//!   cargo run -p hitbox-examples --example hyper_client
//!
//! Expected output:
//!   First request - cache MISS, fetches from httpbin.org
//!   Second request - cache HIT, returns cached response

use std::time::Duration;

use bytes::Bytes;
use http::Request;
use http_body_util::Full;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use tower::{Service, ServiceBuilder, ServiceExt as _};

use hitbox::Config;
use hitbox::policy::PolicyConfig;
use hitbox_http::extractors::{MethodConfig, MethodExtractor};
use hitbox_http::predicates::{NeutralRequestPredicate, NeutralResponsePredicate};
use hitbox_http::request;
use hitbox_moka::MokaBackend;
use hitbox_tower::Cache;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter("info,hitbox=debug")
        .init();

    // Create hyper client
    let client = Client::builder(TokioExecutor::new()).build_http();

    // Create in-memory cache backend
    let backend = MokaBackend::builder().max_entries(100).build();

    // Configure caching: cache all requests, 60 second TTL
    // Request body: Full<Bytes> (what we send)
    // Response body: Incoming (what hyper client receives)
    let config = Config::builder()
        .request_predicate(NeutralRequestPredicate::new())
        .response_predicate(NeutralResponsePredicate::new())
        .extractor(request::extractor().method(MethodConfig::new()))
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    // Build the cache layer
    let cache_layer = Cache::builder().backend(backend).config(config).build();

    // Stack: Cache -> HyperClient
    let mut cached_client = ServiceBuilder::new().layer(cache_layer).service(client);

    // First request - should be a cache MISS
    tracing::info!("Making first request (expect cache MISS)...");
    let req1 = Request::get("http://httpbin.org/get").body(Full::new(Bytes::new()))?;

    let resp1 = cached_client.ready().await?.call(req1).await?;
    let cache_status1 = resp1
        .headers()
        .get("x-cache-status")
        .map(|v| v.to_str().unwrap_or("unknown"))
        .unwrap_or("no header");
    tracing::info!(
        "First response: status={}, cache={}",
        resp1.status(),
        cache_status1
    );

    // Collect response body
    let body1 = resp1
        .into_body()
        .collect()
        .await
        .expect("Failed to collect body");
    tracing::info!("Response body length: {} bytes", body1.len());

    // Second request - should be a cache HIT
    tracing::info!("Making second request (expect cache HIT)...");
    let req2 = Request::get("http://httpbin.org/get").body(Full::new(Bytes::new()))?;

    let resp2 = cached_client.ready().await?.call(req2).await?;
    let cache_status2 = resp2
        .headers()
        .get("x-cache-status")
        .map(|v| v.to_str().unwrap_or("unknown"))
        .unwrap_or("no header");
    tracing::info!(
        "Second response: status={}, cache={}",
        resp2.status(),
        cache_status2
    );

    Ok(())
}
