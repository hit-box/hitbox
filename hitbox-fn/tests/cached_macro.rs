//! Integration tests for #[cached] macro with skip(...) attribute and lifetime support.

use std::time::Duration;

use hitbox::CacheStatus;
use hitbox::policy::PolicyConfig;
use hitbox_derive::cached;
use hitbox_fn::Cache;
use hitbox_moka::MokaBackend;

// =============================================================================
// Types without KeyExtract (for testing skip)
// =============================================================================

/// A type that intentionally does NOT implement KeyExtract.
/// This simulates a database connection or request context.
#[derive(Debug)]
pub struct DbConnection {
    _id: u64,
}

impl DbConnection {
    pub fn new(id: u64) -> Self {
        Self { _id: id }
    }
}

// =============================================================================
// Cached functions with skip(...) on macro
// =============================================================================

/// Function with one skipped parameter and one included.
#[cached(prefix = "compute", skip(_request_id))]
pub async fn compute_with_skip(_request_id: String, value: i64) -> i64 {
    value * 2
}

/// Function with multiple parameters, some skipped.
#[cached(prefix = "multi", skip(_trace_id, _span_id))]
pub async fn multi_params(a: i64, _trace_id: String, b: i64, _span_id: String) -> i64 {
    a + b
}

/// Function with all parameters skipped.
#[cached(prefix = "all_skip", skip(_id1, _id2))]
pub async fn all_params_skipped(_id1: String, _id2: String) -> i64 {
    42
}

/// Function with first parameter skipped.
#[cached(prefix = "first_skip", skip(_skip))]
pub async fn first_param_skipped(_skip: i64, keep: i64) -> i64 {
    keep
}

/// Function with last parameter skipped.
#[cached(prefix = "last_skip", skip(_skip))]
pub async fn last_param_skipped(keep: i64, _skip: i64) -> i64 {
    keep
}

/// Function with a skipped parameter that does NOT implement KeyExtract.
/// This proves that skipped parameters don't need KeyExtract bound.
#[cached(prefix = "with_db", skip(_db))]
pub async fn with_db_connection(_db: DbConnection, user_id: i64) -> String {
    format!("user_{}", user_id)
}

// =============================================================================
// Generic type parameter support
// =============================================================================

use hitbox_core::KeyPart;
use hitbox_fn::KeyExtract;

/// A type that implements KeyExtract for use in generic tests.
#[derive(Debug, Clone)]
pub struct TypedId {
    id: i64,
    label: &'static str,
}

impl TypedId {
    pub fn new(id: i64, label: &'static str) -> Self {
        Self { id, label }
    }
}

impl KeyExtract for TypedId {
    fn extract(&self) -> Vec<KeyPart> {
        vec![
            KeyPart::new("label", Some(self.label.to_string())),
            KeyPart::new("id", Some(self.id.to_string())),
        ]
    }
}

/// Function with a generic type parameter.
#[cached(prefix = "generic")]
pub async fn generic_function<T: KeyExtract + Clone + std::fmt::Debug + Send + Sync + 'static>(
    value: T,
) -> String {
    format!("{:?}", value)
}

/// Function with generic type and skipped parameter.
#[cached(prefix = "generic_skip", skip(_ctx))]
pub async fn generic_with_skip<T: KeyExtract + Clone + std::fmt::Debug + Send + Sync + 'static>(
    _ctx: String,
    value: T,
) -> String {
    format!("{:?}", value)
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

#[tokio::test]
async fn test_skipped_type_without_key_extract() {
    let cache = create_cache();

    // DbConnection does NOT implement KeyExtract, but can be skipped
    let db1 = DbConnection::new(1);
    let db2 = DbConnection::new(2);

    let (r1, c1) = with_db_connection(db1, 42)
        .cache(&cache)
        .with_context()
        .await;
    let (r2, c2) = with_db_connection(db2, 42)
        .cache(&cache)
        .with_context()
        .await;

    // Same user_id = cache hit, despite different DbConnection
    assert_eq!(r1, r2);
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Hit);
}

// =============================================================================
// Generic type parameter tests
// =============================================================================

#[tokio::test]
async fn test_generic_function_same_value() {
    let cache = create_cache();

    let id1 = TypedId::new(42, "user");
    let id2 = TypedId::new(42, "user");

    let (r1, c1) = generic_function(id1).cache(&cache).with_context().await;
    let (r2, c2) = generic_function(id2).cache(&cache).with_context().await;

    // Same value = cache hit
    assert_eq!(r1, r2);
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Hit);
}

#[tokio::test]
async fn test_generic_function_different_value() {
    let cache = create_cache();

    let id1 = TypedId::new(1, "user");
    let id2 = TypedId::new(2, "user");

    let (_, c1) = generic_function(id1).cache(&cache).with_context().await;
    let (_, c2) = generic_function(id2).cache(&cache).with_context().await;

    // Different value = cache miss
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Miss);
}

#[tokio::test]
async fn test_generic_function_different_label() {
    let cache = create_cache();

    // Same id but different label = different cache key
    let id1 = TypedId::new(42, "user");
    let id2 = TypedId::new(42, "product");

    let (_, c1) = generic_function(id1).cache(&cache).with_context().await;
    let (_, c2) = generic_function(id2).cache(&cache).with_context().await;

    // Different label = cache miss
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Miss);
}

#[tokio::test]
async fn test_generic_with_skip() {
    let cache = create_cache();

    let id1 = TypedId::new(42, "user");
    let id2 = TypedId::new(42, "user");

    // Different ctx values should not affect cache key
    let (r1, c1) = generic_with_skip("ctx-1".to_string(), id1)
        .cache(&cache)
        .with_context()
        .await;
    let (r2, c2) = generic_with_skip("ctx-2".to_string(), id2)
        .cache(&cache)
        .with_context()
        .await;

    // Same value, different ctx (skipped) = cache hit
    assert_eq!(r1, r2);
    assert_eq!(c1.status, CacheStatus::Miss);
    assert_eq!(c2.status, CacheStatus::Hit);
}
