//! Integration tests for #[cached] macro with #[key_extract(skip)] on parameters.

use std::time::Duration;

use hitbox::CacheStatus;
use hitbox_fn::Cache;
use hitbox_fn::prelude::*;
use hitbox_moka::MokaBackend;

// =============================================================================
// Cached functions with #[key_extract(skip)] on parameters
// =============================================================================

/// Function with one skipped parameter and one included.
#[cached(prefix = "compute")]
pub async fn compute_with_skip(#[key_extract(skip)] _request_id: String, value: i64) -> i64 {
    value * 2
}

/// Function with multiple parameters, some skipped.
#[cached(prefix = "multi")]
pub async fn multi_params(
    a: i64,
    #[key_extract(skip)] _trace_id: String,
    b: i64,
    #[key_extract(skip)] _span_id: String,
) -> i64 {
    a + b
}

/// Function with all parameters skipped.
#[cached(prefix = "all_skip")]
pub async fn all_params_skipped(
    #[key_extract(skip)] _id1: String,
    #[key_extract(skip)] _id2: String,
) -> i64 {
    42
}

/// Function with first parameter skipped.
#[cached(prefix = "first_skip")]
pub async fn first_param_skipped(#[key_extract(skip)] _skip: i64, keep: i64) -> i64 {
    keep
}

/// Function with last parameter skipped.
#[cached(prefix = "last_skip")]
pub async fn last_param_skipped(keep: i64, #[key_extract(skip)] _skip: i64) -> i64 {
    keep
}

// =============================================================================
// Tests
// =============================================================================

fn create_cache()
-> Cache<MokaBackend, hitbox::concurrency::NoopConcurrencyManager, hitbox_core::DisabledOffload> {
    Cache::builder()
        .backend(MokaBackend::builder().max_entries(100).build())
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build()
}

#[tokio::test]
async fn test_skipped_param_not_in_cache_key() {
    let cache = create_cache();

    // Call with different request_id but same value
    let (r1, c1) = compute_with_skip("req-1".into(), 10)
        .cache(&cache)
        .with_context()
        .await;
    let (r2, c2) = compute_with_skip("req-2".into(), 10)
        .cache(&cache)
        .with_context()
        .await;

    // Both should return same result
    assert_eq!(r1, r2);
    // First should be miss, second should be hit (same cache key)
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Hit);
}

#[tokio::test]
async fn test_included_param_affects_cache_key() {
    let cache = create_cache();

    // Call with same request_id but different value
    let (_, c1) = compute_with_skip("req-1".into(), 10)
        .cache(&cache)
        .with_context()
        .await;
    let (_, c2) = compute_with_skip("req-1".into(), 20)
        .cache(&cache)
        .with_context()
        .await;

    // Both should be misses (different cache keys due to different value)
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Miss);
}

#[tokio::test]
async fn test_multiple_skipped_params() {
    let cache = create_cache();

    // Call with different trace_id and span_id but same a and b
    let (r1, c1) = multi_params(1, "trace-1".into(), 2, "span-1".into())
        .cache(&cache)
        .with_context()
        .await;
    let (r2, c2) = multi_params(1, "trace-2".into(), 2, "span-2".into())
        .cache(&cache)
        .with_context()
        .await;

    assert_eq!(r1, r2);
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Hit);
}

#[tokio::test]
async fn test_multiple_params_different_values() {
    let cache = create_cache();

    // Same trace/span, different a value
    let (_, c1) = multi_params(1, "trace".into(), 2, "span".into())
        .cache(&cache)
        .with_context()
        .await;
    let (_, c2) = multi_params(100, "trace".into(), 2, "span".into())
        .cache(&cache)
        .with_context()
        .await;

    // Different cache keys
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Miss);
}

#[tokio::test]
async fn test_all_params_skipped_same_key() {
    let cache = create_cache();

    // All params skipped - any call should hit same key
    let (r1, c1) = all_params_skipped("a".into(), "b".into())
        .cache(&cache)
        .with_context()
        .await;
    let (r2, c2) = all_params_skipped("x".into(), "y".into())
        .cache(&cache)
        .with_context()
        .await;

    assert_eq!(r1, r2);
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Hit);
}

#[tokio::test]
async fn test_first_param_skipped() {
    let cache = create_cache();

    let (_, c1) = first_param_skipped(999, 42)
        .cache(&cache)
        .with_context()
        .await;
    let (_, c2) = first_param_skipped(111, 42)
        .cache(&cache)
        .with_context()
        .await;

    // Different first param (skipped) - should hit
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Hit);
}

#[tokio::test]
async fn test_last_param_skipped() {
    let cache = create_cache();

    let (_, c1) = last_param_skipped(42, 999)
        .cache(&cache)
        .with_context()
        .await;
    let (_, c2) = last_param_skipped(42, 111)
        .cache(&cache)
        .with_context()
        .await;

    // Different last param (skipped) - should hit
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Hit);
}
