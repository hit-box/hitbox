//! Integration tests for CacheMiddleware using wiremock.

use std::time::Duration;

use hitbox::policy::PolicyConfig;
use hitbox::{Config, Neutral};
use hitbox_http::extractors::Method as MethodExtractor;
use hitbox_http::extractors::path::PathExtractor;
use hitbox_http::predicates::request::Method as MethodPredicate;
use hitbox_http::predicates::request::method::Operation as MethodOp;
use hitbox_moka::MokaBackend;
use hitbox_reqwest::{CacheMiddleware, NoopConcurrencyManager};
use reqwest::Client;
use reqwest_middleware::ClientBuilder;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test 1: Cache MISS then HIT - basic caching works
#[tokio::test]
async fn test_cache_miss_then_hit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/data"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "message": "Hello from server"
        })))
        .expect(1) // Should only be called once due to caching
        .mount(&mock_server)
        .await;

    let backend = MokaBackend::builder().max_entries(100).build();

    let config = Config::builder()
        .request_predicate(MethodPredicate::new(MethodOp::eq(http::Method::GET)))
        .response_predicate(Neutral::new())
        .extractor(MethodExtractor::new().path("/{path}*"))
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    let middleware = CacheMiddleware::builder()
        .backend(backend)
        .config(config)
        .concurrency_manager(NoopConcurrencyManager)
        .build();

    let client = ClientBuilder::new(Client::new()).with(middleware).build();

    let url = format!("{}/data", mock_server.uri());

    // First request - should be a cache miss
    let response1 = client.get(&url).send().await.unwrap();
    assert_eq!(response1.status(), 200);
    assert_eq!(response1.headers().get("X-Cache-Status").unwrap(), "MISS");
    let body1: serde_json::Value = serde_json::from_str(&response1.text().await.unwrap()).unwrap();
    assert_eq!(body1["message"], "Hello from server");

    // Second request - should be a cache hit
    let response2 = client.get(&url).send().await.unwrap();
    assert_eq!(response2.status(), 200);
    assert_eq!(response2.headers().get("X-Cache-Status").unwrap(), "HIT");
    let body2: serde_json::Value = serde_json::from_str(&response2.text().await.unwrap()).unwrap();
    assert_eq!(body2["message"], "Hello from server");
}

/// Test 2: Response integrity - body, headers, status preserved after caching
#[tokio::test]
async fn test_response_integrity() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/headers"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("response body content")
                .insert_header("X-Custom-Header", "custom-value")
                .insert_header("X-Another-Header", "another-value"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let backend = MokaBackend::builder().max_entries(100).build();

    let config = Config::builder()
        .request_predicate(MethodPredicate::new(MethodOp::eq(http::Method::GET)))
        .response_predicate(Neutral::new())
        .extractor(MethodExtractor::new().path("/{path}*"))
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    let middleware = CacheMiddleware::builder()
        .backend(backend)
        .config(config)
        .build();

    let client = ClientBuilder::new(Client::new()).with(middleware).build();

    let url = format!("{}/headers", mock_server.uri());

    // First request - cache miss
    let response1 = client.get(&url).send().await.unwrap();
    assert_eq!(response1.status(), 200);
    assert_eq!(
        response1.headers().get("X-Custom-Header").unwrap(),
        "custom-value"
    );
    assert_eq!(
        response1.headers().get("X-Another-Header").unwrap(),
        "another-value"
    );
    assert_eq!(response1.text().await.unwrap(), "response body content");

    // Second request - cache hit, verify all preserved
    let response2 = client.get(&url).send().await.unwrap();
    assert_eq!(response2.headers().get("X-Cache-Status").unwrap(), "HIT");

    // Status preserved
    assert_eq!(response2.status(), 200);

    // Headers preserved
    assert_eq!(
        response2.headers().get("X-Custom-Header").unwrap(),
        "custom-value"
    );
    assert_eq!(
        response2.headers().get("X-Another-Header").unwrap(),
        "another-value"
    );

    // Body preserved
    assert_eq!(response2.text().await.unwrap(), "response body content");
}

/// Test 3: Body limit exceeded - body > limit not cached, but full body returned
#[tokio::test]
async fn test_body_limit_exceeded_returns_full_body() {
    use hitbox_http::predicates::body::{Body as BodyPredicate, Operation as BodyOperation};

    let mock_server = MockServer::start().await;

    // Create a body larger than the limit (200 bytes > 100 byte limit)
    let large_body = "x".repeat(200);

    Mock::given(method("GET"))
        .and(path("/large"))
        .respond_with(ResponseTemplate::new(200).set_body_string(large_body.clone()))
        .expect(2) // Should be called twice since response won't be cached
        .mount(&mock_server)
        .await;

    let backend = MokaBackend::builder().max_entries(100).build();

    // Configure body limit of 100 bytes using response predicate
    let config = Config::builder()
        .request_predicate(MethodPredicate::new(MethodOp::eq(http::Method::GET)))
        .response_predicate(BodyPredicate::new(BodyOperation::Limit { bytes: 100 }))
        .extractor(MethodExtractor::new().path("/{path}*"))
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    let middleware = CacheMiddleware::builder()
        .backend(backend)
        .config(config)
        .build();

    let client = ClientBuilder::new(Client::new()).with(middleware).build();

    let url = format!("{}/large", mock_server.uri());

    // First request - body exceeds limit, not cached, but full body returned
    let response1 = client.get(&url).send().await.unwrap();
    assert_eq!(response1.status(), 200);
    assert_eq!(response1.headers().get("X-Cache-Status").unwrap(), "MISS");
    let body1 = response1.text().await.unwrap();
    assert_eq!(body1.len(), 200, "Full body should be returned");
    assert_eq!(body1, large_body);

    // Second request - not cached due to body limit, so another MISS
    let response2 = client.get(&url).send().await.unwrap();
    assert_eq!(response2.status(), 200);
    assert_eq!(
        response2.headers().get("X-Cache-Status").unwrap(),
        "MISS",
        "Should be MISS because body exceeded limit"
    );
    let body2 = response2.text().await.unwrap();
    assert_eq!(body2.len(), 200, "Full body should still be returned");
    assert_eq!(body2, large_body);
}
