//! Integration tests for #[cached] macro with skip(...) attribute and lifetime support.

use std::time::Duration;

use hitbox::policy::PolicyConfig;
use hitbox::{CacheStatus, ForwardReason};
use hitbox_derive::{CacheableResponse, cached};
use hitbox_fn::Cache;
use hitbox_moka::MokaBackend;
use serde::{Deserialize, Serialize};

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
// Zero-argument cached function
// =============================================================================

/// Function with no arguments at all.
#[cached]
pub async fn no_args_function() -> i64 {
    42
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

use hitbox::KeyPart;
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
#[cached]
pub async fn generic_function<T: KeyExtract + Clone + std::fmt::Debug + Send + Sync + 'static>(
    value: T,
) -> String {
    format!("{:?}", value)
}

/// Function with generic type and skipped parameter.
#[cached(skip(_ctx))]
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
-> Cache<MokaBackend, hitbox::concurrency::NoopConcurrencyManager, hitbox::DisabledOffload> {
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
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
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
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(c2.status, CacheStatus::Forward(ForwardReason::Miss));
}

#[tokio::test]
async fn test_multiple_skipped_params() {
    let cache = create_cache();

    // Call with different trace_id and span_id but same a and b
    let (r1, c1) = multi_params(1, "trace-1".into(), 2, "span-1".into())
        .cache(&cache)
        .with_context()
        .await;

    // Clone cache and use the clone — exercises Clone for Cache
    let cache2 = cache.clone();
    let (r2, c2) = multi_params(1, "trace-2".into(), 2, "span-2".into())
        .cache(&cache2)
        .with_context()
        .await;

    assert_eq!(r1, r2);
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
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
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(c2.status, CacheStatus::Forward(ForwardReason::Miss));
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
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
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
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
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
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
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
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
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
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
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
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(c2.status, CacheStatus::Forward(ForwardReason::Miss));
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
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(c2.status, CacheStatus::Forward(ForwardReason::Miss));
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
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(c2.status, CacheStatus::Hit);
}

// =============================================================================
// Reference parameter support
// =============================================================================

/// Function with a reference parameter.
#[cached(prefix = "ref_param")]
pub async fn with_reference<'a>(data: &'a str) -> String {
    data.to_uppercase()
}

/// Function with reference and owned parameters.
#[cached(prefix = "ref_mixed")]
pub async fn with_mixed_params<'a>(prefix: &'a str, id: i64) -> String {
    format!("{}_{}", prefix, id)
}

/// Function with skipped reference parameter.
#[cached(prefix = "ref_skip", skip(_ctx))]
pub async fn with_skipped_reference<'a>(_ctx: &'a str, value: i64) -> i64 {
    value * 2
}

#[tokio::test]
async fn test_reference_param() {
    let cache = create_cache();

    let data = String::from("hello");
    let (r1, c1) = with_reference(&data).cache(&cache).with_context().await;
    let (r2, c2) = with_reference(&data).cache(&cache).with_context().await;

    assert_eq!(r1, "HELLO");
    assert_eq!(r2, "HELLO");
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(c2.status, CacheStatus::Hit);
}

#[tokio::test]
async fn test_reference_different_values() {
    let cache = create_cache();

    let (_, c1) = with_reference("hello").cache(&cache).with_context().await;
    let (_, c2) = with_reference("world").cache(&cache).with_context().await;

    // Different values = different cache keys
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(c2.status, CacheStatus::Forward(ForwardReason::Miss));
}

#[tokio::test]
async fn test_mixed_ref_and_owned() {
    let cache = create_cache();

    let prefix = "user";
    let (r1, c1) = with_mixed_params(prefix, 42)
        .cache(&cache)
        .with_context()
        .await;
    let (r2, c2) = with_mixed_params(prefix, 42)
        .cache(&cache)
        .with_context()
        .await;

    assert_eq!(r1, "user_42");
    assert_eq!(r2, "user_42");
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(c2.status, CacheStatus::Hit);
}

#[tokio::test]
async fn test_skipped_reference() {
    let cache = create_cache();

    // Different context (skipped) should hit same cache key
    let (r1, c1) = with_skipped_reference("ctx-1", 21)
        .cache(&cache)
        .with_context()
        .await;
    let (r2, c2) = with_skipped_reference("ctx-2", 21)
        .cache(&cache)
        .with_context()
        .await;

    assert_eq!(r1, 42);
    assert_eq!(r2, 42);
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(c2.status, CacheStatus::Hit);
}

// =============================================================================
// Multiple lifetime parameters
// =============================================================================

#[cached]
pub async fn with_two_lifetimes<'a, 'b>(prefix: &'a str, suffix: &'b str) -> String {
    format!("{}-{}", prefix, suffix)
}

#[tokio::test]
async fn test_two_lifetimes_passthrough() {
    let p = String::from("hello");
    let s = String::from("world");
    let result = with_two_lifetimes(&p, &s).await;
    assert_eq!(result, "hello-world");
}

#[tokio::test]
async fn test_two_lifetimes_different_scopes() {
    // The two references have genuinely different lifetimes:
    // `"long"` is 'static while `&s` borrows a local.
    // This exercises the synthetic '__hitbox lifetime (inferred as the shorter one).
    let result = {
        let s = String::from("short");
        with_two_lifetimes("long", &s).await
    };
    assert_eq!(result, "long-short");
}

#[tokio::test]
async fn test_two_lifetimes_cached() {
    let cache = create_cache();

    let p = String::from("key");
    let s = String::from("val");

    let (r1, c1) = with_two_lifetimes(&p, &s)
        .cache(&cache)
        .with_context()
        .await;
    let (r2, c2) = with_two_lifetimes(&p, &s)
        .cache(&cache)
        .with_context()
        .await;

    assert_eq!(r1, "key-val");
    assert_eq!(r2, "key-val");
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(c2.status, CacheStatus::Hit);
}

#[tokio::test]
async fn test_two_lifetimes_different_values() {
    let cache = create_cache();

    let (r1, c1) = with_two_lifetimes("a", "b")
        .cache(&cache)
        .with_context()
        .await;
    let (r2, c2) = with_two_lifetimes("a", "c")
        .cache(&cache)
        .with_context()
        .await;

    assert_eq!(r1, "a-b");
    assert_eq!(r2, "a-c");
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(c2.status, CacheStatus::Forward(ForwardReason::Miss));
}

// =============================================================================
// CacheableResponse skip field tests
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, CacheableResponse)]
pub struct AuthResult {
    pub user_id: u64,
    pub permissions: Vec<String>,
    #[cacheable_response(skip)]
    pub access_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthError;

#[cached]
pub async fn authenticate(user_id: i64) -> Result<AuthResult, AuthError> {
    Ok(AuthResult {
        user_id: user_id as u64,
        permissions: vec!["read".into(), "write".into()],
        access_token: Some("secret-token".into()),
    })
}

#[tokio::test]
async fn test_skipped_response_field_preserved_on_miss() {
    let cache = create_cache();

    let (result, ctx) = authenticate(1).cache(&cache).with_context().await;

    assert_eq!(ctx.status, CacheStatus::Forward(ForwardReason::Miss));
    let auth = result.unwrap();
    assert_eq!(auth.access_token, Some("secret-token".into()));
    assert_eq!(auth.permissions, vec!["read", "write"]);
}

#[tokio::test]
async fn test_skipped_response_field_default_on_hit() {
    let cache = create_cache();

    // First call — miss, populates cache
    let (r1, c1) = authenticate(2).cache(&cache).with_context().await;
    // Second call — hit, from cache
    let (r2, c2) = authenticate(2).cache(&cache).with_context().await;

    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(c2.status, CacheStatus::Hit);

    // On miss: skipped field preserved
    assert_eq!(
        r1.as_ref().unwrap().access_token,
        Some("secret-token".into())
    );

    // On hit: skipped field is Default (None for Option<String>)
    assert_eq!(r2.as_ref().unwrap().access_token, None);

    // Non-skipped fields are identical
    assert_eq!(r1.as_ref().unwrap().user_id, r2.as_ref().unwrap().user_id);
    assert_eq!(
        r1.as_ref().unwrap().permissions,
        r2.as_ref().unwrap().permissions
    );
}

// =============================================================================
// Skipped field does NOT require Clone
// =============================================================================

/// A type that implements Default but NOT Clone.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv_format",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct NonCloneable {
    pub value: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, CacheableResponse)]
pub struct ResponseWithNonCloneable {
    pub id: u64,
    #[cacheable_response(skip)]
    pub ctx: NonCloneable,
}

#[cached(prefix = "non_clone")]
pub async fn get_with_non_cloneable(id: i64) -> Result<ResponseWithNonCloneable, AuthError> {
    Ok(ResponseWithNonCloneable {
        id: id as u64,
        ctx: NonCloneable {
            value: "original".into(),
        },
    })
}

#[tokio::test]
async fn test_skipped_field_no_clone_bound() {
    let cache = create_cache();

    // Miss: NonCloneable field preserved despite not implementing Clone
    let (r1, c1) = get_with_non_cloneable(1).cache(&cache).with_context().await;
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(r1.as_ref().unwrap().ctx.value, "original");

    // Hit: NonCloneable field is Default
    let (r2, c2) = get_with_non_cloneable(1).cache(&cache).with_context().await;
    assert_eq!(c2.status, CacheStatus::Hit);
    assert_eq!(r2.as_ref().unwrap().ctx.value, "");

    // Non-skipped field identical
    assert_eq!(r1.as_ref().unwrap().id, r2.as_ref().unwrap().id);
}

// =============================================================================
// Zero-argument function tests
// =============================================================================

#[tokio::test]
async fn test_zero_args_always_same_key() {
    use std::sync::Arc;

    let cache = Arc::new(create_cache());

    // Using Arc<Cache> exercises CacheAccess for Arc<T>
    let (r1, c1) = no_args_function().cache(&cache).with_context().await;
    let (r2, c2) = no_args_function().cache(&cache).with_context().await;

    assert_eq!(r1, r2);
    assert_eq!(c1.status, CacheStatus::Forward(ForwardReason::Miss));
    assert_eq!(c2.status, CacheStatus::Hit);
}

#[tokio::test]
async fn test_zero_args_generated_key() {
    use hitbox::Extractor;
    use hitbox_fn::{Arg, Args, FnExtractor, Skipped};

    // Exercise Args::new() and Args::into_inner()
    let args = Args::new(());
    assert_eq!(args.into_inner(), ());

    // Exercise Arg::value()
    let arg = Arg::new("x", 42i64);
    assert_eq!(*arg.value(), 42);

    // Exercise Skipped::value()
    let skipped = Skipped::new("ctx");
    assert_eq!(*skipped.value(), "ctx");

    let extractor = FnExtractor::<Args<()>>::new("no_args_function");
    let (_, key) = extractor.get(Args(())).await.into_cache_key();

    // Zero-arg function should produce key with only the function name
    assert_eq!(key.to_string(), "fn=no_args_function");
}

// =============================================================================
// Passthrough tests (no backend, no policy — direct function call)
// =============================================================================

#[tokio::test]
async fn test_passthrough_no_backend() {
    let result = compute_with_skip("req-1".into(), 10).await;
    assert_eq!(result, 20);
}

#[tokio::test]
async fn test_passthrough_zero_args() {
    let result = no_args_function().await;
    assert_eq!(result, 42);
}

#[tokio::test]
async fn test_passthrough_generic() {
    let id = TypedId::new(42, "user");
    let result = generic_function(id).await;
    assert!(result.contains("42"));
}

// =============================================================================
// Inline .backend().policy() path tests
// =============================================================================

#[tokio::test]
async fn test_inline_backend_policy() {
    let backend = MokaBackend::builder().max_entries(100).build();
    let policy = PolicyConfig::builder().ttl(Duration::from_secs(60)).build();

    let result = compute_with_skip("req-1".into(), 10)
        .backend(backend)
        .policy(policy)
        .await;

    assert_eq!(result, 20);
}

#[tokio::test]
async fn test_inline_backend_policy_with_context() {
    let backend = MokaBackend::builder().max_entries(100).build();
    let policy = PolicyConfig::builder().ttl(Duration::from_secs(60)).build();

    let (result, ctx) = compute_with_skip("req-1".into(), 10)
        .backend(backend)
        .policy(policy)
        .with_context()
        .await;

    assert_eq!(result, 20);
    assert_eq!(ctx.status, CacheStatus::Forward(ForwardReason::Miss));
}

// =============================================================================
// Spy backend for key inspection
// =============================================================================

mod spy_backend {
    use async_trait::async_trait;
    use dashmap::DashMap;
    use hitbox::backend::{Backend, BackendError, CacheBackend, DeleteStatus};
    use hitbox::{CacheKey, CacheValue, Raw};
    use hitbox_backend::format::RonFormat;

    /// A backend that always misses but records keys and values.
    /// Uses RON format for human-readable value inspection.
    pub struct SpyBackend {
        store: DashMap<String, CacheValue<Raw>>,
    }

    impl SpyBackend {
        pub fn new() -> Self {
            Self {
                store: DashMap::new(),
            }
        }

        /// Returns all stored keys as Display strings.
        pub fn keys(&self) -> Vec<String> {
            self.store.iter().map(|e| e.key().clone()).collect()
        }

        /// Returns the raw value bytes for a key, deserialized as RON string.
        pub fn value_as_ron(&self, key: &str) -> Option<String> {
            self.store.get(key).map(|entry| {
                let (_, raw) = entry.value().clone().into_parts();
                String::from_utf8(raw.to_vec()).unwrap()
            })
        }
    }

    #[async_trait]
    impl Backend for SpyBackend {
        async fn read(&self, _key: &CacheKey) -> Result<Option<CacheValue<Raw>>, BackendError> {
            Ok(None)
        }

        async fn write(&self, key: &CacheKey, value: CacheValue<Raw>) -> Result<(), BackendError> {
            self.store.insert(key.to_string(), value);
            Ok(())
        }

        async fn remove(&self, _key: &CacheKey) -> Result<DeleteStatus, BackendError> {
            Ok(DeleteStatus::Missing)
        }

        fn value_format(&self) -> &dyn hitbox_backend::format::Format {
            &RonFormat
        }
    }

    impl CacheBackend for SpyBackend {}
}

// =============================================================================
// Key verification tests (using SpyBackend)
// =============================================================================

use spy_backend::SpyBackend;

#[tokio::test]
async fn test_key_zero_args() {
    let cache = Cache::builder()
        .backend(SpyBackend::new())
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    no_args_function().cache(&cache).await;

    let keys = cache.backend().keys();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0], "fn=no_args_function");

    // Reconstruct a second cache from accessors of the first one
    let cache2 = Cache::builder()
        .backend(SpyBackend::new())
        .policy(cache.policy().as_ref().clone())
        .concurrency_manager(cache.concurrency_manager().clone())
        .offload(*cache.offload())
        .build();

    no_args_function().cache(&cache2).await;

    let keys2 = cache2.backend().keys();
    assert_eq!(keys2.len(), 1);
    assert_eq!(keys2[0], "fn=no_args_function");
}

#[tokio::test]
async fn test_key_with_skip() {
    let cache = Cache::builder()
        .backend(SpyBackend::new())
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    compute_with_skip("req-1".into(), 10).cache(&cache).await;

    let keys = cache.backend().keys();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0], "fn=compute&value=10");
}

#[tokio::test]
async fn test_key_multiple_params_with_skip() {
    let cache = Cache::builder()
        .backend(SpyBackend::new())
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    multi_params(1, "trace-1".into(), 2, "span-1".into())
        .cache(&cache)
        .await;

    let keys = cache.backend().keys();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0], "fn=multi&a=1&b=2");
}

// =============================================================================
// Value verification tests (using SpyBackend with RON format)
// =============================================================================

#[tokio::test]
async fn test_value_i64() {
    let cache = Cache::builder()
        .backend(SpyBackend::new())
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    no_args_function().cache(&cache).await;

    let ron = cache.backend().value_as_ron("fn=no_args_function").unwrap();
    assert_eq!(ron, "42");
}

#[tokio::test]
async fn test_value_computed() {
    let cache = Cache::builder()
        .backend(SpyBackend::new())
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    compute_with_skip("req-1".into(), 10).cache(&cache).await;

    let ron = cache.backend().value_as_ron("fn=compute&value=10").unwrap();
    assert_eq!(ron, "20");
}

#[tokio::test]
async fn test_value_string() {
    let cache = Cache::builder()
        .backend(SpyBackend::new())
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    with_db_connection(DbConnection::new(1), 42)
        .cache(&cache)
        .await;

    let ron = cache
        .backend()
        .value_as_ron("fn=with_db&user_id=42")
        .unwrap();
    assert_eq!(ron, "\"user_42\"");
}

#[tokio::test]
async fn test_value_skipped_response_field() {
    let cache = Cache::builder()
        .backend(SpyBackend::new())
        .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
        .build();

    let _ = authenticate(1).cache(&cache).await;

    let ron = cache
        .backend()
        .value_as_ron("fn=authenticate&user_id=1")
        .unwrap();

    // The cached value should contain user_id and permissions
    assert!(ron.contains("user_id:1"));
    assert!(ron.contains("permissions:[\"read\",\"write\"]"));

    // The skipped field (access_token) is excluded from serialization entirely
    assert!(!ron.contains("access_token"));
}
