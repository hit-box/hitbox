//! Tests for OffloadManager deduplication through the Offload trait interface.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use hitbox::offload::{OffloadConfig, OffloadManager};
use hitbox_core::{CacheKey, Offload, OffloadKey};
use tokio::sync::Barrier;

/// Helper to call register through trait bound (tests the trait interface).
fn register_task<'a, O: Offload<'a>>(
    offload: &O,
    key: impl Into<OffloadKey>,
    future: impl std::future::Future<Output = ()> + Send + 'a,
) {
    offload.register(key, future);
}

#[tokio::test]
async fn test_deduplication_with_named_key() {
    let config = OffloadConfig::builder().deduplicate(true).build();
    let manager = OffloadManager::new(config);

    let counter = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(2));

    let cache_key = CacheKey::from_str("test", "key1");

    // First task - should be spawned
    let counter1 = counter.clone();
    let barrier1 = barrier.clone();
    let spawned1 = manager.register((cache_key.clone(), "revalidate"), async move {
        counter1.fetch_add(1, Ordering::SeqCst);
        barrier1.wait().await;
    });
    assert!(spawned1, "First task should be spawned");

    // Second task with same key - should be deduplicated
    let counter2 = counter.clone();
    let spawned2 = manager.register((cache_key.clone(), "revalidate"), async move {
        counter2.fetch_add(1, Ordering::SeqCst);
    });
    assert!(!spawned2, "Second task should be deduplicated");

    // Release the first task
    barrier.wait().await;
    manager.wait_all().await;

    // Only the first task should have executed
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_register_via_trait_interface() {
    let config = OffloadConfig::builder().deduplicate(true).build();
    let manager = OffloadManager::new(config);

    let counter = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(2));

    let counter_clone = counter.clone();
    let barrier_clone = barrier.clone();

    // Register task through trait interface helper
    register_task(&manager, OffloadKey::explicit("test", 1), async move {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        barrier_clone.wait().await;
    });

    barrier.wait().await;

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_register_with_tuple_syntax() {
    let manager = OffloadManager::default();

    let counter = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(2));

    let cache_key = CacheKey::from_str("user", "123");

    let counter_clone = counter.clone();
    let barrier_clone = barrier.clone();

    // Register using tuple syntax (CacheKey, &str) -> OffloadKey::Named
    register_task(&manager, (cache_key, "revalidate"), async move {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        barrier_clone.wait().await;
    });

    barrier.wait().await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_generated_keys_not_deduplicated() {
    let config = OffloadConfig::builder().deduplicate(true).build();
    let manager = OffloadManager::new(config);

    let counter = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(4)); // 3 tasks + test

    // Multiple tasks with different generated keys should all run
    for i in 0..3 {
        let counter = counter.clone();
        let barrier = barrier.clone();
        let spawned = manager.register(OffloadKey::explicit("cleanup", i), async move {
            counter.fetch_add(1, Ordering::SeqCst);
            barrier.wait().await;
        });
        assert!(spawned, "Generated key task {} should be spawned", i);
    }

    barrier.wait().await;
    manager.wait_all().await;

    // All tasks should have executed
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_different_cache_keys_not_deduplicated() {
    let config = OffloadConfig::builder().deduplicate(true).build();
    let manager = OffloadManager::new(config);

    let counter = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(3)); // 2 tasks + test

    let key1 = CacheKey::from_str("test", "key1");
    let key2 = CacheKey::from_str("test", "key2");

    // Task with key1
    let counter1 = counter.clone();
    let barrier1 = barrier.clone();
    let spawned1 = manager.register((key1, "revalidate"), async move {
        counter1.fetch_add(1, Ordering::SeqCst);
        barrier1.wait().await;
    });
    assert!(spawned1);

    // Task with key2 - different key, should not be deduplicated
    let counter2 = counter.clone();
    let barrier2 = barrier.clone();
    let spawned2 = manager.register((key2, "revalidate"), async move {
        counter2.fetch_add(1, Ordering::SeqCst);
        barrier2.wait().await;
    });
    assert!(spawned2, "Different cache key should not be deduplicated");

    barrier.wait().await;
    manager.wait_all().await;

    // Both tasks should have executed
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_same_cache_key_different_kind_not_deduplicated() {
    let config = OffloadConfig::builder().deduplicate(true).build();
    let manager = OffloadManager::new(config);

    let counter = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(3)); // 2 tasks + test

    let cache_key = CacheKey::from_str("test", "key1");

    // Task with kind "revalidate"
    let counter1 = counter.clone();
    let barrier1 = barrier.clone();
    let spawned1 = manager.register((cache_key.clone(), "revalidate"), async move {
        counter1.fetch_add(1, Ordering::SeqCst);
        barrier1.wait().await;
    });
    assert!(spawned1);

    // Task with same cache_key but different kind "warmup"
    let counter2 = counter.clone();
    let barrier2 = barrier.clone();
    let spawned2 = manager.register((cache_key, "warmup"), async move {
        counter2.fetch_add(1, Ordering::SeqCst);
        barrier2.wait().await;
    });
    assert!(spawned2, "Different kind should not be deduplicated");

    barrier.wait().await;
    manager.wait_all().await;

    // Both tasks should execute (different kinds = different keys)
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_deduplication_disabled_allows_duplicates() {
    let config = OffloadConfig::builder().deduplicate(false).build();
    let manager = OffloadManager::new(config);

    let counter = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(3)); // 2 tasks + test

    let cache_key = CacheKey::from_str("test", "key1");

    // First task
    let counter1 = counter.clone();
    let barrier1 = barrier.clone();
    let spawned1 = manager.register((cache_key.clone(), "revalidate"), async move {
        counter1.fetch_add(1, Ordering::SeqCst);
        barrier1.wait().await;
    });
    assert!(spawned1);

    // Second task with same key - should NOT be deduplicated when disabled
    let counter2 = counter.clone();
    let barrier2 = barrier.clone();
    let spawned2 = manager.register((cache_key, "revalidate"), async move {
        counter2.fetch_add(1, Ordering::SeqCst);
        barrier2.wait().await;
    });
    assert!(spawned2, "Should spawn when deduplication is disabled");

    barrier.wait().await;
    manager.wait_all().await;

    // Both tasks should have executed
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_deduplication_after_task_completes() {
    let config = OffloadConfig::builder().deduplicate(true).build();
    let manager = OffloadManager::new(config);

    let counter = Arc::new(AtomicUsize::new(0));
    let cache_key = CacheKey::from_str("test", "key1");

    // First task
    let counter1 = counter.clone();
    manager.register((cache_key.clone(), "revalidate"), async move {
        counter1.fetch_add(1, Ordering::SeqCst);
    });

    // Wait for first task to complete
    manager.wait_all().await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Second task with same key - should be spawned since first completed
    let counter2 = counter.clone();
    let spawned = manager.register((cache_key, "revalidate"), async move {
        counter2.fetch_add(1, Ordering::SeqCst);
    });
    assert!(spawned, "Task should spawn after previous completed");

    manager.wait_all().await;

    // Both tasks should have executed
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_clone_shares_deduplication_state() {
    let config = OffloadConfig::builder().deduplicate(true).build();
    let manager1 = OffloadManager::new(config);
    let manager2 = manager1.clone();

    let counter = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(2));

    let cache_key = CacheKey::from_str("test", "key1");

    // Register through first clone
    let counter1 = counter.clone();
    let barrier1 = barrier.clone();
    let spawned1 = manager1.register((cache_key.clone(), "revalidate"), async move {
        counter1.fetch_add(1, Ordering::SeqCst);
        barrier1.wait().await;
    });
    assert!(spawned1);

    // Register same key through second clone - should be deduplicated
    let counter2 = counter.clone();
    let spawned2 = manager2.register((cache_key, "revalidate"), async move {
        counter2.fetch_add(1, Ordering::SeqCst);
    });
    assert!(!spawned2, "Should deduplicate across clones");

    barrier.wait().await;
    manager1.wait_all().await;

    // Only first task should have executed
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_is_in_flight() {
    let manager = OffloadManager::default();
    let barrier = Arc::new(Barrier::new(2));

    let cache_key = CacheKey::from_str("test", "key1");
    let key = OffloadKey::keyed(cache_key, "revalidate");

    assert!(
        !manager.is_in_flight(&key),
        "Should not be in flight initially"
    );

    let barrier_clone = barrier.clone();
    manager.register(key.clone(), async move {
        barrier_clone.wait().await;
    });

    assert!(
        manager.is_in_flight(&key),
        "Should be in flight after register"
    );

    barrier.wait().await;
    manager.wait_all().await;

    assert!(
        !manager.is_in_flight(&key),
        "Should not be in flight after completion"
    );
}

#[tokio::test]
async fn test_active_task_count() {
    let manager = OffloadManager::default();
    let barrier = Arc::new(Barrier::new(4)); // 3 tasks + test

    assert_eq!(manager.active_task_count(), 0);

    for i in 0..3 {
        let barrier = barrier.clone();
        manager.register(OffloadKey::explicit("test", i), async move {
            barrier.wait().await;
        });
    }

    // Give tasks time to start
    tokio::task::yield_now().await;
    assert_eq!(manager.active_task_count(), 3);

    barrier.wait().await;
    manager.wait_all().await;

    manager.cleanup_finished();
    assert_eq!(manager.active_task_count(), 0);
}
