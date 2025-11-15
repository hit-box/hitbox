//! Integration tests for cache lock mechanism (dogpile effect prevention)

use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use hitbox::policy::{EnabledCacheConfig, LockConfig, PolicyConfig};
use hitbox_moka::MokaBackend;
use hitbox_tower::Cache;
use hitbox_configuration::Endpoint;
use axum::{Router, routing::get, body::Body, extract::State};
use axum_test::TestServer;

/// Shared state to track upstream fetch count
#[derive(Clone)]
struct AppState {
    fetch_count: Arc<AtomicUsize>,
}

/// Handler that simulates upstream fetch with delay
async fn mock_handler(State(state): State<AppState>) -> &'static str {
    // Increment fetch counter
    state.fetch_count.fetch_add(1, Ordering::SeqCst);

    // Simulate network delay
    tokio::time::sleep(Duration::from_millis(100)).await;

    "mock response"
}

/// Handler that simulates variable latency - first request slow, second fast
async fn variable_latency_handler(State(state): State<AppState>) -> String {
    // Get current count and increment
    let count = state.fetch_count.fetch_add(1, Ordering::SeqCst);

    if count == 0 {
        // First request: slow (300ms), returns "300"
        tokio::time::sleep(Duration::from_millis(300)).await;
        "300".to_string()
    } else {
        // Second+ request: fast (10ms), returns "10"
        tokio::time::sleep(Duration::from_millis(10)).await;
        "10".to_string()
    }
}

#[tokio::test]
async fn test_dogpile_prevention_concurrency_1() {
    // Setup: Create backend and fetch counter
    let backend = MokaBackend::builder(100).build();
    let fetch_count = Arc::new(AtomicUsize::new(0));

    // Configure cache with locks enabled (concurrency = 1)
    let endpoint_config = Endpoint::<Body, Body> {
        policy: PolicyConfig::Enabled(EnabledCacheConfig {
            ttl: Some(60),
            stale: None,
            locks: LockConfig::Enabled { concurrency: 1 },
        }),
        ..Default::default()
    };

    // Build the cache layer
    let cache = Cache::builder()
        .backend(backend)
        .config(endpoint_config)
        .build();

    // Create app state
    let app_state = AppState {
        fetch_count: fetch_count.clone(),
    };

    // Create a simple router with the cache layer
    let app = Router::new()
        .route("/test/path", get(mock_handler))
        .with_state(app_state)
        .layer(cache);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Spawn 10 concurrent requests for the same resource
    let num_requests = 10;
    let mut futures = Vec::new();

    for _ in 0..num_requests {
        let fut = async { server.get("/test/path").await };
        futures.push(fut);
    }

    // Wait for all requests to complete
    let responses = futures::future::join_all(futures).await;

    // Verify all requests succeeded
    assert_eq!(responses.len(), num_requests);
    for response in responses {
        response.assert_status_ok();
    }

    // The critical assertion: Only 1 upstream fetch should have occurred
    // because all requests were for the same cache key with concurrency=1
    let actual_fetch_count = fetch_count.load(Ordering::SeqCst);
    assert_eq!(
        actual_fetch_count,
        1,
        "Expected exactly 1 upstream fetch with concurrency=1, but got {}",
        actual_fetch_count
    );
}

#[tokio::test]
async fn test_no_locks_multiple_fetches() {
    // Setup: Create backend and fetch counter
    let backend = MokaBackend::builder(100).build();
    let fetch_count = Arc::new(AtomicUsize::new(0));

    // Configure cache WITHOUT locks
    let endpoint_config = Endpoint::<Body, Body> {
        policy: PolicyConfig::Enabled(EnabledCacheConfig {
            ttl: Some(60),
            stale: None,
            locks: LockConfig::Disabled,
        }),
        ..Default::default()
    };

    // Build the cache layer
    let cache = Cache::builder()
        .backend(backend)
        .config(endpoint_config)
        .build();

    // Create app state
    let app_state = AppState {
        fetch_count: fetch_count.clone(),
    };

    // Create a simple router with the cache layer
    let app = Router::new()
        .route("/test/path", get(mock_handler))
        .with_state(app_state)
        .layer(cache);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Spawn 10 concurrent requests for the same resource
    let num_requests = 10;
    let mut futures = Vec::new();

    for _ in 0..num_requests {
        let fut = async { server.get("/test/path").await };
        futures.push(fut);
    }

    // Wait for all requests to complete
    let responses = futures::future::join_all(futures).await;

    // Verify all requests succeeded
    assert_eq!(responses.len(), num_requests);
    for response in responses {
        response.assert_status_ok();
    }

    // Without locks, all requests should hit upstream (dogpile effect)
    let actual_fetch_count = fetch_count.load(Ordering::SeqCst);
    assert_eq!(
        actual_fetch_count,
        num_requests,
        "Expected {} upstream fetches without locks, but got {}",
        num_requests,
        actual_fetch_count
    );
}

#[tokio::test]
async fn test_dogpile_prevention_concurrency_2() {
    // Setup: Create backend and fetch counter
    let backend = MokaBackend::builder(100).build();
    let fetch_count = Arc::new(AtomicUsize::new(0));

    // Configure cache with locks enabled (concurrency = 2)
    let endpoint_config = Endpoint::<Body, Body> {
        policy: PolicyConfig::Enabled(EnabledCacheConfig {
            ttl: Some(60),
            stale: None,
            locks: LockConfig::Enabled { concurrency: 2 },
        }),
        ..Default::default()
    };

    // Build the cache layer
    let cache = Cache::builder()
        .backend(backend)
        .config(endpoint_config)
        .build();

    // Create app state
    let app_state = AppState {
        fetch_count: fetch_count.clone(),
    };

    // Create a simple router with the cache layer
    let app = Router::new()
        .route("/test/path", get(mock_handler))
        .with_state(app_state)
        .layer(cache);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Spawn 10 concurrent requests for the same resource
    let num_requests = 10;
    let mut futures = Vec::new();

    for _ in 0..num_requests {
        let fut = async { server.get("/test/path").await };
        futures.push(fut);
    }

    // Wait for all requests to complete
    let responses = futures::future::join_all(futures).await;

    // Verify all requests succeeded
    assert_eq!(responses.len(), num_requests);
    for response in responses {
        response.assert_status_ok();
    }

    // The critical assertion: Only 2 upstream fetches should have occurred
    // because concurrency=2 allows 2 concurrent fetches
    let actual_fetch_count = fetch_count.load(Ordering::SeqCst);
    assert_eq!(
        actual_fetch_count,
        2,
        "Expected exactly 2 upstream fetches with concurrency=2, but got {}",
        actual_fetch_count
    );
}

#[tokio::test]
async fn test_concurrency_2_fast_request_unblocks_waiting() {
    // Setup: Create backend and fetch counter
    let backend = MokaBackend::builder(100).build();
    let fetch_count = Arc::new(AtomicUsize::new(0));

    // Configure cache with locks enabled (concurrency = 2)
    let endpoint_config = Endpoint::<Body, Body> {
        policy: PolicyConfig::Enabled(EnabledCacheConfig {
            ttl: Some(60),
            stale: None,
            locks: LockConfig::Enabled { concurrency: 2 },
        }),
        ..Default::default()
    };

    // Build the cache layer
    let cache = Cache::builder()
        .backend(backend)
        .config(endpoint_config)
        .build();

    // Create app state
    let app_state = AppState {
        fetch_count: fetch_count.clone(),
    };

    // Create a simple router with the cache layer and variable latency handler
    let app = Router::new()
        .route("/test/path", get(variable_latency_handler))
        .with_state(app_state)
        .layer(cache);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Spawn 10 concurrent requests for the same resource
    let num_requests = 10;
    let mut futures = Vec::new();

    for _ in 0..num_requests {
        let fut = async { server.get("/test/path").await };
        futures.push(fut);
    }

    // Wait for all requests to complete
    let responses = futures::future::join_all(futures).await;

    // Verify all requests succeeded
    assert_eq!(responses.len(), num_requests);

    // Collect response bodies
    let mut bodies = Vec::new();
    for response in responses {
        response.assert_status_ok();
        let body = response.text();
        bodies.push(body);
    }

    // Count how many responses have "300" vs "10"
    let count_300 = bodies.iter().filter(|b| *b == "300").count();
    let count_10 = bodies.iter().filter(|b| *b == "10").count();

    // Expected: 1 slow response with "300", 9 fast/cached responses with "10"
    // This proves that the fast request (10ms) completed first and cached its result,
    // allowing the 8 waiting requests to get the cached "10" instead of waiting 300ms
    assert_eq!(
        count_300, 1,
        "Expected 1 response with '300' (slow request), but got {}",
        count_300
    );
    assert_eq!(
        count_10, 9,
        "Expected 9 responses with '10' (fast request + 8 cached), but got {}",
        count_10
    );

    // Verify only 2 upstream fetches occurred
    let actual_fetch_count = fetch_count.load(Ordering::SeqCst);
    assert_eq!(
        actual_fetch_count,
        2,
        "Expected exactly 2 upstream fetches with concurrency=2, but got {}",
        actual_fetch_count
    );
}
