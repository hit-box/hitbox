//! Tests for composition read policies (Sequential, Race, Parallel).

use bytes::Bytes;
use hitbox_backend::Backend;
use hitbox_backend::composition::policy::{
    CompositionReadPolicy, ParallelReadPolicy, RaceReadPolicy, SequentialReadPolicy,
};
use hitbox_core::{BoxContext, CacheContext, CacheKey, CacheValue};

use crate::common::{ErrorBackend, TestBackend, TestOffloadManager};

/// Helper to create a default context for tests
fn default_ctx() -> BoxContext {
    CacheContext::default().boxed()
}

// =============================================================================
// SequentialReadPolicy Tests
// =============================================================================

#[tokio::test]
async fn test_sequential_l1_hit() {
    let policy = SequentialReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("from_l1"), None, None, None);

    l1.write(&key, value.clone()).await.unwrap();

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l1"));
}

#[tokio::test]
async fn test_sequential_l2_hit() {
    let policy = SequentialReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("from_l2"), None, None, None);

    l2.write(&key, value.clone()).await.unwrap();

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l2"));
}

#[tokio::test]
async fn test_sequential_both_miss() {
    let policy = SequentialReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_none());
}

#[tokio::test]
async fn test_sequential_l1_error_l2_hit() {
    let policy = SequentialReadPolicy::new();
    let l1 = ErrorBackend;
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("from_l2"), None, None, None);

    l2.write(&key, value.clone()).await.unwrap();

    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l2"));
}

// =============================================================================
// RaceReadPolicy Tests
// =============================================================================

#[tokio::test]
async fn test_race_l1_hit() {
    let policy = RaceReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("from_l1"), None, None, None);

    l1.write(&key, value.clone()).await.unwrap();

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l1"));
}

#[tokio::test]
async fn test_race_l2_hit() {
    let policy = RaceReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("from_l2"), None, None, None);

    l2.write(&key, value.clone()).await.unwrap();

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l2"));
}

#[tokio::test]
async fn test_race_both_miss() {
    let policy = RaceReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_none());
}

#[tokio::test]
async fn test_race_l1_error_l2_hit() {
    let policy = RaceReadPolicy::new();
    let l1 = ErrorBackend;
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("from_l2"), None, None, None);

    l2.write(&key, value.clone()).await.unwrap();

    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l2"));
}

// =============================================================================
// ParallelReadPolicy Tests
// =============================================================================

#[tokio::test]
async fn test_parallel_both_hit_prefer_l1() {
    let policy = ParallelReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");

    l1.write(
        &key,
        CacheValue::new(Bytes::from("from_l1"), None, None, None),
    )
    .await
    .unwrap();
    l2.write(
        &key,
        CacheValue::new(Bytes::from("from_l2"), None, None, None),
    )
    .await
    .unwrap();

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    // Both have no expiry - should prefer L1 (tie-breaker)
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l1"));
}

#[tokio::test]
async fn test_parallel_l1_miss_l2_hit() {
    let policy = ParallelReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("from_l2"), None, None, None);

    l2.write(&key, value.clone()).await.unwrap();

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l2"));
}

#[tokio::test]
async fn test_parallel_both_miss() {
    let policy = ParallelReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_none());
}

#[tokio::test]
async fn test_parallel_l1_error_l2_hit() {
    let policy = ParallelReadPolicy::new();
    let l1 = ErrorBackend;
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("from_l2"), None, None, None);

    l2.write(&key, value.clone()).await.unwrap();

    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l2"));
}

// =============================================================================
// ParallelReadPolicy TTL-based Selection Tests
// =============================================================================

#[tokio::test]
async fn test_parallel_both_hit_l2_fresher_ttl() {
    use chrono::Utc;

    let policy = ParallelReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let now = Utc::now();

    // L1 has shorter TTL (expires in 10 seconds)
    let l1_value = CacheValue::new(
        Bytes::from("from_l1"),
        Some(now + chrono::Duration::seconds(10)),
        None,
        None,
    );

    // L2 has longer TTL (expires in 60 seconds)
    let l2_value = CacheValue::new(
        Bytes::from("from_l2"),
        Some(now + chrono::Duration::seconds(60)),
        None,
        None,
    );

    l1.write(&key, l1_value).await.unwrap();
    l2.write(&key, l2_value).await.unwrap();

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    // Should prefer L2 (fresher/longer TTL)
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l2"));
}

#[tokio::test]
async fn test_parallel_both_hit_l1_fresher_ttl() {
    use chrono::Utc;

    let policy = ParallelReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let now = Utc::now();

    // L1 has longer TTL (expires in 60 seconds)
    let l1_value = CacheValue::new(
        Bytes::from("from_l1"),
        Some(now + chrono::Duration::seconds(60)),
        None,
        None,
    );

    // L2 has shorter TTL (expires in 10 seconds)
    let l2_value = CacheValue::new(
        Bytes::from("from_l2"),
        Some(now + chrono::Duration::seconds(10)),
        None,
        None,
    );

    l1.write(&key, l1_value).await.unwrap();
    l2.write(&key, l2_value).await.unwrap();

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    // Should prefer L1 (fresher/longer TTL)
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l1"));
}

#[tokio::test]
async fn test_parallel_both_hit_equal_ttl() {
    use chrono::Utc;

    let policy = ParallelReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let now = Utc::now();
    let expiry = now + chrono::Duration::seconds(30);

    // Both have same TTL
    let l1_value = CacheValue::new(Bytes::from("from_l1"), Some(expiry), None, None);

    let l2_value = CacheValue::new(Bytes::from("from_l2"), Some(expiry), None, None);

    l1.write(&key, l1_value).await.unwrap();
    l2.write(&key, l2_value).await.unwrap();

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    // Equal TTLs - should prefer L1 (tie-breaker)
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l1"));
}

#[tokio::test]
async fn test_parallel_both_hit_l2_no_expiry() {
    use chrono::Utc;

    let policy = ParallelReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let now = Utc::now();

    // L1 has expiry
    let l1_value = CacheValue::new(
        Bytes::from("from_l1"),
        Some(now + chrono::Duration::seconds(60)),
        None,
        None,
    );

    // L2 has no expiry (infinite TTL)
    let l2_value = CacheValue::new(Bytes::from("from_l2"), None, None, None);

    l1.write(&key, l1_value).await.unwrap();
    l2.write(&key, l2_value).await.unwrap();

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    // L2 has no expiry (infinite) - should prefer L2
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l2"));
}

#[tokio::test]
async fn test_parallel_both_hit_l1_no_expiry() {
    use chrono::Utc;

    let policy = ParallelReadPolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let now = Utc::now();

    // L1 has no expiry (infinite TTL)
    let l1_value = CacheValue::new(Bytes::from("from_l1"), None, None, None);

    // L2 has expiry
    let l2_value = CacheValue::new(
        Bytes::from("from_l2"),
        Some(now + chrono::Duration::seconds(60)),
        None,
        None,
    );

    l1.write(&key, l1_value).await.unwrap();
    l2.write(&key, l2_value).await.unwrap();

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let read_l1 = |k: CacheKey| async move { (l1_clone.read(&k).await, default_ctx()) };
    let read_l2 = |k: CacheKey| async move { (l2_clone.read(&k).await, default_ctx()) };

    let read_result = policy
        .execute_with(key, read_l1, read_l2, &offload)
        .await
        .unwrap();

    assert!(read_result.value.is_some());
    // L1 has no expiry (infinite) - should prefer L1
    assert_eq!(read_result.value.unwrap().data(), &Bytes::from("from_l1"));
}
