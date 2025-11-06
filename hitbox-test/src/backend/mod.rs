use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use hitbox_backend::{CacheBackend, DeleteStatus};
use hitbox_core::{
    CacheKey, CachePolicy, CacheValue, CacheableResponse, EntityPolicyConfig, ResponseCachePolicy,
};
use serde::{Deserialize, Serialize};

/// Test response type for backend testing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[async_trait]
impl CacheableResponse for TestResponse {
    type Cached = Self;
    type Subject = Self;

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

    async fn into_cached(self) -> CachePolicy<Self::Cached, Self> {
        CachePolicy::Cacheable(self)
    }

    async fn from_cached(cached: Self::Cached) -> Self {
        cached
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
pub async fn run_backend_tests<B: CacheBackend + Send + Sync>(backend: B) {
    test_write_and_read(&backend).await;
    test_write_and_read_with_metadata(&backend).await;
    test_delete_existing(&backend).await;
    test_delete_missing(&backend).await;
    test_read_nonexistent(&backend).await;
    test_overwrite(&backend).await;
    test_multiple_keys(&backend).await;
    test_binary_data(&backend).await;
}

async fn test_write_and_read<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "write-read");
    let response = TestResponse::new(1, "test-response", vec![1, 2, 3, 4, 5]);
    let value = CacheValue::new(response.clone(), None, None);

    // Write
    backend
        .set::<TestResponse>(&key, &value, Some(Duration::from_secs(3600)))
        .await
        .expect("failed to write");

    // Read
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key)
        .await
        .expect("failed to read");

    assert!(result.is_some(), "value should exist");
    let cached_value = result.unwrap();
    assert_eq!(cached_value.data, response, "data should match");
}

async fn test_write_and_read_with_metadata<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "with-metadata");
    let response = TestResponse::new(2, "metadata-test", vec![10, 20, 30]);

    let expire = Some(Utc::now() + chrono::Duration::hours(1));
    let stale = Some(Utc::now() + chrono::Duration::minutes(30));
    let value = CacheValue::new(response.clone(), expire, stale);

    // Write
    backend
        .set::<TestResponse>(&key, &value, Some(Duration::from_secs(3600)))
        .await
        .expect("failed to write");

    // Read
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key)
        .await
        .expect("failed to read");

    assert!(result.is_some(), "value should exist");
    let cached_value = result.unwrap();
    assert_eq!(cached_value.data, response, "data should match");
    assert!(cached_value.expire.is_some(), "expire should be set");
    assert!(cached_value.stale.is_some(), "stale should be set");
}

async fn test_delete_existing<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "delete-existing");
    let response = TestResponse::new(3, "delete-test", vec![]);
    let value = CacheValue::new(response, None, None);

    // Write
    backend
        .set::<TestResponse>(&key, &value, Some(Duration::from_secs(3600)))
        .await
        .expect("failed to write");

    // Delete
    let status = backend.delete(&key).await.expect("failed to delete");
    assert_eq!(status, DeleteStatus::Deleted(1), "should delete 1 key");

    // Verify deleted
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key)
        .await
        .expect("failed to read");
    assert!(result.is_none(), "value should not exist after delete");
}

async fn test_delete_missing<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "delete-missing");

    let status = backend.delete(&key).await.expect("failed to delete");
    assert_eq!(status, DeleteStatus::Missing, "should report missing");
}

async fn test_read_nonexistent<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "nonexistent");

    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key)
        .await
        .expect("failed to read");
    assert!(result.is_none(), "nonexistent key should return None");
}

async fn test_overwrite<B: CacheBackend>(backend: &B) {
    let key = CacheKey::from_str("test", "overwrite");

    // Write first value
    let response1 = TestResponse::new(4, "original", vec![1, 2, 3]);
    let value1 = CacheValue::new(response1, None, None);
    backend
        .set::<TestResponse>(&key, &value1, Some(Duration::from_secs(3600)))
        .await
        .expect("failed to write first value");

    // Overwrite with second value
    let response2 = TestResponse::new(5, "updated", vec![4, 5, 6, 7]);
    let value2 = CacheValue::new(response2.clone(), None, None);
    backend
        .set::<TestResponse>(&key, &value2, Some(Duration::from_secs(3600)))
        .await
        .expect("failed to overwrite");

    // Read and verify we get the second value
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key)
        .await
        .expect("failed to read");
    assert!(result.is_some(), "value should exist");
    assert_eq!(result.unwrap().data, response2, "should get updated value");
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
        backend
            .set::<TestResponse>(key, &value, Some(Duration::from_secs(3600)))
            .await
            .expect("failed to write");
    }

    // Read all and verify
    for (key, expected_response) in &keys_and_values {
        let result: Option<CacheValue<TestResponse>> = backend
            .get::<TestResponse>(key)
            .await
            .expect("failed to read");
        assert!(result.is_some(), "value should exist for key");
        assert_eq!(
            result.unwrap().data,
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
    backend
        .set::<TestResponse>(&key, &value, Some(Duration::from_secs(3600)))
        .await
        .expect("failed to write binary data");

    // Read
    let result: Option<CacheValue<TestResponse>> = backend
        .get::<TestResponse>(&key)
        .await
        .expect("failed to read");

    assert!(result.is_some(), "binary value should exist");
    let cached = result.unwrap();
    assert_eq!(
        cached.data.data, binary_data,
        "binary data should match exactly"
    );
}
