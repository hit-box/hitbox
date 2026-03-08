//! Tests for composition write policies (Sequential, OptimisticParallel).

use bytes::Bytes;
use hitbox_backend::Backend;
use hitbox_backend::composition::policy::{
    CompositionWritePolicy, OptimisticParallelWritePolicy, SequentialWritePolicy,
};
use hitbox_core::{CacheKey, CacheValue};

use crate::common::{ErrorBackend, TestBackend, TestOffloadManager};

// =============================================================================
// SequentialWritePolicy Tests
// =============================================================================

#[tokio::test]
async fn test_sequential_both_succeed() {
    let policy = SequentialWritePolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("test_value"), None, None);

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let value_clone1 = value.clone();
    let value_clone2 = value.clone();

    let write_l1 = |k: CacheKey| async move { l1_clone.write(&k, value_clone1).await };
    let write_l2 = |k: CacheKey| async move { l2_clone.write(&k, value_clone2).await };

    let result = policy
        .execute_with(key.clone(), write_l1, write_l2, &offload)
        .await;

    assert!(result.is_ok());
    assert!(l1.has(&key));
    assert!(l2.has(&key));
}

#[tokio::test]
async fn test_sequential_l1_fails() {
    let policy = SequentialWritePolicy::new();
    let l1 = ErrorBackend;
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("test_value"), None, None);

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let value_clone1 = value.clone();
    let value_clone2 = value.clone();

    let write_l1 = |k: CacheKey| async move { l1_clone.write(&k, value_clone1).await };
    let write_l2 = |k: CacheKey| async move { l2_clone.write(&k, value_clone2).await };

    let result = policy
        .execute_with(key.clone(), write_l1, write_l2, &offload)
        .await;

    // Should fail - L1 failed, so L2 is never called
    assert!(result.is_err());
    assert!(!l2.has(&key)); // L2 should not be written to
}

#[tokio::test]
async fn test_sequential_l2_fails() {
    let policy = SequentialWritePolicy::new();
    let l1 = TestBackend::new();
    let l2 = ErrorBackend;
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("test_value"), None, None);

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let value_clone1 = value.clone();
    let value_clone2 = value.clone();

    let write_l1 = |k: CacheKey| async move { l1_clone.write(&k, value_clone1).await };
    let write_l2 = |k: CacheKey| async move { l2_clone.write(&k, value_clone2).await };

    let result = policy
        .execute_with(key.clone(), write_l1, write_l2, &offload)
        .await;

    // Should fail - L2 failed (inconsistent state)
    assert!(result.is_err());
    assert!(l1.has(&key)); // L1 was written successfully
    // L2 failed, so inconsistent state
}

#[tokio::test]
async fn test_sequential_both_fail() {
    let policy = SequentialWritePolicy::new();
    let l1 = ErrorBackend;
    let l2 = ErrorBackend;
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("test_value"), None, None);

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let value_clone1 = value.clone();
    let value_clone2 = value.clone();

    let write_l1 = |k: CacheKey| async move { l1_clone.write(&k, value_clone1).await };
    let write_l2 = |k: CacheKey| async move { l2_clone.write(&k, value_clone2).await };

    let result = policy.execute_with(key, write_l1, write_l2, &offload).await;

    // Should fail - L1 failed, L2 never called
    assert!(result.is_err());
}

// =============================================================================
// OptimisticParallelWritePolicy Tests
// =============================================================================

#[tokio::test]
async fn test_optimistic_parallel_both_succeed() {
    let policy = OptimisticParallelWritePolicy::new();
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("test_value"), None, None);

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let value_clone1 = value.clone();
    let value_clone2 = value.clone();

    let write_l1 = |k: CacheKey| async move { l1_clone.write(&k, value_clone1).await };
    let write_l2 = |k: CacheKey| async move { l2_clone.write(&k, value_clone2).await };

    let result = policy
        .execute_with(key.clone(), write_l1, write_l2, &offload)
        .await;

    assert!(result.is_ok());
    assert!(l1.has(&key));
    assert!(l2.has(&key));
}

#[tokio::test]
async fn test_optimistic_parallel_l1_fails_l2_succeeds() {
    let policy = OptimisticParallelWritePolicy::new();
    let l1 = ErrorBackend;
    let l2 = TestBackend::new();
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("test_value"), None, None);

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let value_clone1 = value.clone();
    let value_clone2 = value.clone();

    let write_l1 = |k: CacheKey| async move { l1_clone.write(&k, value_clone1).await };
    let write_l2 = |k: CacheKey| async move { l2_clone.write(&k, value_clone2).await };

    let result = policy
        .execute_with(key.clone(), write_l1, write_l2, &offload)
        .await;

    // Should SUCCEED - L2 succeeded (optimistic)
    assert!(result.is_ok());
    assert!(l2.has(&key));
}

#[tokio::test]
async fn test_optimistic_parallel_l1_succeeds_l2_fails() {
    let policy = OptimisticParallelWritePolicy::new();
    let l1 = TestBackend::new();
    let l2 = ErrorBackend;
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("test_value"), None, None);

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let value_clone1 = value.clone();
    let value_clone2 = value.clone();

    let write_l1 = |k: CacheKey| async move { l1_clone.write(&k, value_clone1).await };
    let write_l2 = |k: CacheKey| async move { l2_clone.write(&k, value_clone2).await };

    let result = policy
        .execute_with(key.clone(), write_l1, write_l2, &offload)
        .await;

    // Should SUCCEED - L1 succeeded (optimistic)
    assert!(result.is_ok());
    assert!(l1.has(&key));
}

#[tokio::test]
async fn test_optimistic_parallel_both_fail() {
    let policy = OptimisticParallelWritePolicy::new();
    let l1 = ErrorBackend;
    let l2 = ErrorBackend;
    let offload = TestOffloadManager;

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(Bytes::from("test_value"), None, None);

    let l1_clone = l1.clone();
    let l2_clone = l2.clone();
    let value_clone1 = value.clone();
    let value_clone2 = value.clone();

    let write_l1 = |k: CacheKey| async move { l1_clone.write(&k, value_clone1).await };
    let write_l2 = |k: CacheKey| async move { l2_clone.write(&k, value_clone2).await };

    let result = policy.execute_with(key, write_l1, write_l2, &offload).await;

    // Should FAIL - both failed
    assert!(result.is_err());
}
