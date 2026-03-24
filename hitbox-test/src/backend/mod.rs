use std::future::Ready;

use chrono::{DateTime, Utc};
use hitbox_backend::format::FormatExt;
use hitbox_backend::{Backend, CacheBackend, CacheKeyFormat, DeleteStatus};
use hitbox_core::{
    BoxContext, CacheContext, CacheKey, CachePolicy, CacheValue, CacheableResponse,
    EntityPolicyConfig, ResponseCachePolicy,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Test response type for backend testing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "rkyv_format",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct TestResponse {
    pub id: u64,
    pub name: String,
    pub data: Vec<u8>,
}

impl TestResponse {
    pub fn new(id: u64, name: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            id,
            name: name.into(),
            data,
        }
    }
}

impl CacheableResponse for TestResponse {
    type Cached = Self;
    type Subject = Self;
    type IntoCachedFuture = Ready<CachePolicy<Self::Cached, Self>>;
    type FromCachedFuture = Ready<Self>;

    async fn cache_policy<P>(
        self,
        _predicates: P,
        _config: &EntityPolicyConfig,
    ) -> ResponseCachePolicy<Self>
    where
        P: hitbox_core::Predicate<Subject = Self::Subject> + Send + Sync,
    {
        // Always cacheable for testing
        CachePolicy::Cacheable(CacheValue::new(self.clone(), None, None))
    }

    fn into_cached(self) -> Self::IntoCachedFuture {
        std::future::ready(CachePolicy::Cacheable(self))
    }

    fn from_cached(cached: Self::Cached) -> Self::FromCachedFuture {
        std::future::ready(cached)
    }
}

/// Generic backend test suite
///
/// This function runs a comprehensive test suite against any backend implementation.
/// It tests all core backend functionality including:
/// - Write and read operations
/// - Serialization with different formats
/// - TTL expiration
/// - Delete operations
/// - Missing key handling
/// - Metadata (expire/stale) preservation
pub async fn run_backend_tests<B: CacheBackend + Send + Sync>(backend: &B) {
    test_write_and_read(backend).await;
    test_write_and_read_with_metadata(backend).await;
    test_delete_existing(backend).await;
    test_delete_missing(backend).await;
    test_read_nonexistent(backend).await;
    test_overwrite(backend).await;
    test_multiple_keys(backend).await;
    test_binary_data(backend).await;
    test_expire_metadata_exact_match(backend).await;
    test_stale_metadata_exact_match(backend).await;
    test_expire_and_stale_combined(backend).await;
    test_no_metadata(backend).await;
}

async fn test_write_and_read<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "write-read");
    let response = TestResponse::new(1, "test-response", vec![1, 2, 3, 4, 5]);
    let value = CacheValue::new(response.clone(), None, None);

    // Write
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value, &mut ctx)
        .await
        .expect("failed to write");

    // Read
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to read");

    assert!(result.is_some(), "value should exist");
    let cached_value = result.unwrap();
    assert_eq!(*cached_value.data(), response, "data should match");
}

async fn test_write_and_read_with_metadata<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "with-metadata");
    let response = TestResponse::new(2, "metadata-test", vec![10, 20, 30]);

    let expire = Some(Utc::now() + chrono::Duration::hours(1));
    let stale = Some(Utc::now() + chrono::Duration::minutes(30));
    let value = CacheValue::new(response.clone(), expire, stale);

    // Write
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value, &mut ctx)
        .await
        .expect("failed to write");

    // Read
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to read");

    assert!(result.is_some(), "value should exist");
    let cached_value = result.unwrap();
    assert_eq!(*cached_value.data(), response, "data should match");
    assert!(cached_value.expire().is_some(), "expire should be set");
    assert!(cached_value.stale().is_some(), "stale should be set");
}

async fn test_delete_existing<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "delete-existing");
    let response = TestResponse::new(3, "delete-test", vec![]);
    let value = CacheValue::new(response, None, None);

    // Write
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value, &mut ctx)
        .await
        .expect("failed to write");

    // Delete
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let status = backend
        .delete(&key, &mut ctx)
        .await
        .expect("failed to delete");
    assert_eq!(status, DeleteStatus::Deleted(1), "should delete 1 key");

    // Verify deleted
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to read");
    assert!(result.is_none(), "value should not exist after delete");
}

async fn test_delete_missing<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "delete-missing");

    let mut ctx: BoxContext = CacheContext::default().boxed();
    let status = backend
        .delete(&key, &mut ctx)
        .await
        .expect("failed to delete");
    assert_eq!(status, DeleteStatus::Missing, "should report missing");
}

async fn test_read_nonexistent<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "nonexistent");

    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to read");
    assert!(result.is_none(), "nonexistent key should return None");
}

async fn test_overwrite<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "overwrite");

    // Write first value
    let response1 = TestResponse::new(4, "original", vec![1, 2, 3]);
    let value1 = CacheValue::new(response1, None, None);
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value1, &mut ctx)
        .await
        .expect("failed to write first value");

    // Overwrite with second value
    let response2 = TestResponse::new(5, "updated", vec![4, 5, 6, 7]);
    let value2 = CacheValue::new(response2.clone(), None, None);
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value2, &mut ctx)
        .await
        .expect("failed to overwrite");

    // Read and verify we get the second value
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to read");
    assert!(result.is_some(), "value should exist");
    assert_eq!(
        *result.unwrap().data(),
        response2,
        "should get updated value"
    );
}

async fn test_multiple_keys<B: CacheBackend>(backend: &B) {
    let keys_and_values = vec![
        (
            CacheKey::from_str("test", "multi-1"),
            TestResponse::new(10, "first", vec![1]),
        ),
        (
            CacheKey::from_str("test", "multi-2"),
            TestResponse::new(20, "second", vec![2, 2]),
        ),
        (
            CacheKey::from_str("test", "multi-3"),
            TestResponse::new(30, "third", vec![3, 3, 3]),
        ),
    ];

    // Write all
    for (key, response) in &keys_and_values {
        let value = CacheValue::new(response.clone(), None, None);
        let mut ctx: BoxContext = CacheContext::default().boxed();
        backend
            .set::<TestResponse>(key, &value, &mut ctx)
            .await
            .expect("failed to write");
    }

    // Read all and verify
    for (key, expected_response) in &keys_and_values {
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result: Option<CacheValue<TestResponse>> = backend
            .get::<TestResponse>(key, &mut ctx)
            .await
            .expect("failed to read");
        assert!(result.is_some(), "value should exist for key");
        assert_eq!(
            *result.unwrap().data(),
            *expected_response,
            "data should match for key"
        );
    }
}

async fn test_binary_data<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "binary");

    // Create response with various binary data
    let binary_data: Vec<u8> = (0..=255).collect();
    let response = TestResponse::new(99, "binary-test", binary_data.clone());
    let value = CacheValue::new(response.clone(), None, None);

    // Write
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value, &mut ctx)
        .await
        .expect("failed to write binary data");

    // Read
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to read");

    assert!(result.is_some(), "binary value should exist");
    let cached = result.unwrap();
    assert_eq!(
        cached.data().data,
        binary_data,
        "binary data should match exactly"
    );
}

// =============================================================================
// TTL/Stale Metadata Tests
// =============================================================================

/// Maximum allowed drift for expire time comparisons (1 second).
/// Some backends (like Redis) derive expire from TTL, causing small drift.
const EXPIRE_DRIFT_TOLERANCE_MS: i64 = 1000;

/// Check if two DateTimes are within tolerance (for expire comparisons)
fn expire_times_match(actual: Option<DateTime<Utc>>, expected: Option<DateTime<Utc>>) -> bool {
    match (actual, expected) {
        (Some(a), Some(e)) => (a - e).num_milliseconds().abs() <= EXPIRE_DRIFT_TOLERANCE_MS,
        (None, None) => true,
        _ => false,
    }
}

/// Check if two DateTimes match at millisecond precision.
/// Some backends store timestamps at millisecond precision, losing sub-ms data.
fn stale_times_match(actual: Option<DateTime<Utc>>, expected: Option<DateTime<Utc>>) -> bool {
    match (actual, expected) {
        (Some(a), Some(e)) => a.timestamp_millis() == e.timestamp_millis(),
        (None, None) => true,
        _ => false,
    }
}

async fn test_expire_metadata_exact_match<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "expire-exact");
    let response = TestResponse::new(200, "expire-test", vec![1, 2, 3]);

    // Use a specific expire time
    let expire_time = Utc::now() + chrono::Duration::seconds(3600);
    let value = CacheValue::new(response.clone(), Some(expire_time), None);

    // Write
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value, &mut ctx)
        .await
        .expect("failed to write");

    // Read and verify expire time (with tolerance for TTL-based backends)
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to read");

    assert!(result.is_some(), "value should exist");
    let cached = result.unwrap();
    assert_eq!(*cached.data(), response, "data should match");
    assert!(
        expire_times_match(cached.expire(), Some(expire_time)),
        "expire time should match (within {}ms tolerance): actual={:?}, expected={:?}",
        EXPIRE_DRIFT_TOLERANCE_MS,
        cached.expire(),
        expire_time
    );
    assert!(cached.stale().is_none(), "stale should be None");
}

async fn test_stale_metadata_exact_match<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "stale-exact");
    let response = TestResponse::new(201, "stale-test", vec![4, 5, 6]);

    // Use specific expire and stale times
    let expire_time = Utc::now() + chrono::Duration::seconds(3600);
    let stale_time = Utc::now() + chrono::Duration::seconds(1800);
    let value = CacheValue::new(response.clone(), Some(expire_time), Some(stale_time));

    // Write
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value, &mut ctx)
        .await
        .expect("failed to write");

    // Read and verify exact stale time
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to read");

    assert!(result.is_some(), "value should exist");
    let cached = result.unwrap();
    assert_eq!(*cached.data(), response, "data should match");
    assert!(
        stale_times_match(cached.stale(), Some(stale_time)),
        "stale time should match (at ms precision): actual={:?}, expected={:?}",
        cached.stale(),
        stale_time
    );
    assert!(
        expire_times_match(cached.expire(), Some(expire_time)),
        "expire time should match (within tolerance): actual={:?}, expected={:?}",
        cached.expire(),
        expire_time
    );
}

async fn test_expire_and_stale_combined<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "expire-stale-combined");
    let response = TestResponse::new(202, "combined-test", vec![7, 8, 9]);

    // Set expire far in the future, stale closer
    let expire_time = Utc::now() + chrono::Duration::hours(24);
    let stale_time = Utc::now() + chrono::Duration::hours(1);
    let value = CacheValue::new(response.clone(), Some(expire_time), Some(stale_time));

    // Write
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value, &mut ctx)
        .await
        .expect("failed to write");

    // Read and verify both metadata fields
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to read");

    assert!(result.is_some(), "value should exist");
    let cached = result.unwrap();
    assert_eq!(*cached.data(), response, "data should match");
    assert!(
        expire_times_match(cached.expire(), Some(expire_time)),
        "expire time should match (within tolerance): actual={:?}, expected={:?}",
        cached.expire(),
        expire_time
    );
    assert!(
        stale_times_match(cached.stale(), Some(stale_time)),
        "stale time should match (at ms precision): actual={:?}, expected={:?}",
        cached.stale(),
        stale_time
    );

    // Verify stale < expire (logical consistency)
    assert!(
        stale_time < expire_time,
        "stale time should be before expire time"
    );
}

async fn test_no_metadata<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "no-metadata");
    let response = TestResponse::new(203, "no-metadata-test", vec![10, 11, 12]);

    // No expire, no stale
    let value = CacheValue::new(response.clone(), None, None);

    // Write
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value, &mut ctx)
        .await
        .expect("failed to write");

    // Read and verify no metadata
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to read");

    assert!(result.is_some(), "value should exist");
    let cached = result.unwrap();
    assert_eq!(*cached.data(), response, "data should match");
    assert!(cached.expire().is_none(), "expire should be None");
    assert!(cached.stale().is_none(), "stale should be None");
}

/// Test UrlEncoded key + JSON value format
pub async fn test_url_encoded_key_json_value<B: Backend + CacheBackend>(backend: &B) {
    // Verify backend key format configuration
    assert_eq!(
        backend.key_format(),
        &CacheKeyFormat::UrlEncoded,
        "backend should use UrlEncoded key format"
    );

    let key = CacheKey::from_str("format-test", "url-json");
    let response = TestResponse::new(100, "url-json-test", vec![1, 2, 3]);
    let value = CacheValue::new(response.clone(), None, None);

    // Write and read
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value, &mut ctx)
        .await
        .expect("failed to write");

    // Read raw to verify format
    let raw_value = backend
        .read(&key)
        .await
        .expect("failed to read raw")
        .expect("value should exist");

    // Decompress the data before validating format
    let decompressed = backend
        .compressor()
        .decompress(Cow::Borrowed(raw_value.data()))
        .expect("failed to decompress");

    // Verify it's valid JSON
    let as_string =
        String::from_utf8(decompressed.into_owned()).expect("Value should be valid UTF-8 JSON");
    assert!(
        as_string.contains("\"id\"") || as_string.contains("id"),
        "Value should contain JSON fields"
    );

    // Verify can deserialize
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to deserialize");
    assert!(result.is_some());
    assert_eq!(*result.unwrap().data(), response);
}

/// Test UrlEncoded key + Bincode value format
pub async fn test_url_encoded_key_bincode_value<B: Backend + CacheBackend>(backend: &B) {
    assert_eq!(
        backend.key_format(),
        &CacheKeyFormat::UrlEncoded,
        "backend should use UrlEncoded key format"
    );

    let key = CacheKey::from_str("format-test", "url-bincode");
    let response = TestResponse::new(101, "url-bincode-test", vec![4, 5, 6]);
    let value = CacheValue::new(response.clone(), None, None);

    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value, &mut ctx)
        .await
        .expect("failed to write");

    // Read raw to verify format
    let raw_value = backend
        .read(&key)
        .await
        .expect("failed to read raw")
        .expect("value should exist");

    // Decompress the data before validating format
    let decompressed = backend
        .compressor()
        .decompress(Cow::Borrowed(raw_value.data()))
        .expect("failed to decompress");

    // Verify it's NOT readable JSON (binary format)
    let as_string = String::from_utf8(decompressed.into_owned());
    assert!(
        as_string.is_err() || !as_string.unwrap().contains("\"id\""),
        "Value should be in Bincode format (binary), not JSON"
    );

    // Verify can deserialize
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to deserialize");
    assert!(result.is_some());
    assert_eq!(*result.unwrap().data(), response);
}

/// Test Bitcode key + JSON value format
pub async fn test_bitcode_key_json_value<B: Backend + CacheBackend>(backend: &B) {
    assert_eq!(
        backend.key_format(),
        &CacheKeyFormat::Bitcode,
        "backend should use Bitcode key format"
    );

    let key = CacheKey::from_str("format-test", "bitcode-json");
    let response = TestResponse::new(102, "bitcode-json-test", vec![7, 8, 9]);
    let value = CacheValue::new(response.clone(), None, None);

    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value, &mut ctx)
        .await
        .expect("failed to write");

    // Read raw to verify value format
    let raw_value = backend
        .read(&key)
        .await
        .expect("failed to read raw")
        .expect("value should exist");

    // Decompress the data before validating format
    let decompressed = backend
        .compressor()
        .decompress(Cow::Borrowed(raw_value.data()))
        .expect("failed to decompress");

    // Verify value is JSON
    let as_string =
        String::from_utf8(decompressed.into_owned()).expect("Value should be valid UTF-8 JSON");
    assert!(
        as_string.contains("\"id\"") || as_string.contains("id"),
        "Value should be in JSON format"
    );

    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to deserialize");
    assert!(result.is_some());
    assert_eq!(*result.unwrap().data(), response);
}

/// Test Bitcode key + Bincode value format
pub async fn test_bitcode_key_bincode_value<B: Backend + CacheBackend>(backend: &B) {
    assert_eq!(
        backend.key_format(),
        &CacheKeyFormat::Bitcode,
        "backend should use Bitcode key format"
    );

    let key = CacheKey::from_str("format-test", "bitcode-bincode");
    let response = TestResponse::new(103, "bitcode-bincode-test", vec![10, 11, 12]);
    let value = CacheValue::new(response.clone(), None, None);

    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value, &mut ctx)
        .await
        .expect("failed to write");

    // Read raw to verify value format
    let raw_value = backend
        .read(&key)
        .await
        .expect("failed to read raw")
        .expect("value should exist");

    // Decompress the data before validating format
    let decompressed = backend
        .compressor()
        .decompress(Cow::Borrowed(raw_value.data()))
        .expect("failed to decompress");

    // Verify value is binary Bincode
    let as_string = String::from_utf8(decompressed.into_owned());
    assert!(
        as_string.is_err() || !as_string.unwrap().contains("\"id\""),
        "Value should be in Bincode format (binary), not JSON"
    );

    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to deserialize");
    assert!(result.is_some());
    assert_eq!(*result.unwrap().data(), response);
}

/// Test that compression is actually being used
///
/// This test verifies that compression is working by comparing the serialized data
/// with the raw stored data. If they're different, compression was applied.
///
/// # Arguments
/// * `backend` - Backend configured with a compressor
pub async fn test_compression_is_used<B>(backend: &B)
where
    B: Backend + CacheBackend,
{
    // Create a large, highly compressible test response with lots of repeated data
    let large_repeated_data = vec![42u8; 10000]; // 10KB of the same byte
    let key = CacheKey::from_str("compression-test", "verify-compression");
    let response = TestResponse::new(999, "compression-test-data", large_repeated_data);
    let value = CacheValue::new(response.clone(), None, None);

    // Serialize the value to get the raw uncompressed serialized bytes
    let ctx = CacheContext::default();
    let serialized = backend
        .value_format()
        .serialize(&response, &ctx)
        .expect("failed to serialize");

    // Write to backend (should apply compression via compressor)
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestResponse>(&key, &value, &mut ctx)
        .await
        .expect("failed to write to backend");

    // Read raw stored bytes
    let raw_value = backend
        .read(&key)
        .await
        .expect("failed to read from backend")
        .expect("value should exist in backend");

    // Verify compression was applied: raw stored bytes should differ from serialized bytes
    assert_ne!(
        *raw_value.data(),
        serialized,
        "Stored data should be different from serialized data if compression is applied"
    );

    // Verify backend can correctly deserialize the data
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key, &mut ctx)
        .await
        .expect("failed to deserialize from backend");
    assert!(result.is_some(), "value should exist after compression");
    assert_eq!(
        *result.unwrap().data(),
        response,
        "data should be identical after compression roundtrip"
    );
}
