//! Tests for memory-based cache eviction.

use bytes::Bytes;
use chrono::Utc;
use hitbox::backend::Backend;
use hitbox_core::{CacheKey, CacheValue, KeyPart};
use hitbox_moka::{EvictionPolicy, MokaBackend, MokaBackendBuilder};

/// Create a cache key with predictable size.
fn make_key(id: u32) -> CacheKey {
    CacheKey::new("test", 1, vec![KeyPart::new("id", Some(id.to_string()))])
}

/// Create a cache value with specified data size.
fn make_value(size: usize) -> CacheValue<Bytes> {
    let data = Bytes::from(vec![0u8; size]);
    let expire = Some(Utc::now() + chrono::Duration::hours(1));
    CacheValue::new(data, expire, None, None)
}

/// Calculate total entry size (key + value).
fn entry_size(key: &CacheKey, value: &CacheValue<Bytes>) -> usize {
    key.memory_size() + value.memory_size()
}

#[tokio::test]
async fn test_max_bytes_eviction_precise() {
    // Calculate actual entry size
    let key = make_key(1);
    let value = make_value(100);
    let single_entry_size = entry_size(&key, &value);

    // Set capacity for exactly 3 entries
    let capacity = single_entry_size * 3;
    // Use MokaBackendBuilder::default() to test Default impl
    let backend = MokaBackendBuilder::default()
        .max_bytes(capacity as u64)
        .build();

    // Insert 3 entries - should all fit
    for i in 1..=3 {
        let key = make_key(i);
        let value = make_value(100);
        backend.write(&key, value).await.unwrap();
    }

    backend.cache().run_pending_tasks().await;

    // All 3 should be present
    for i in 1..=3 {
        let key = make_key(i);
        assert!(
            backend.read(&key).await.unwrap().is_some(),
            "Entry {} should exist (capacity fits 3)",
            i
        );
    }

    // Insert 4th entry - should trigger eviction of oldest entry
    let key4 = make_key(4);
    let value4 = make_value(100);
    backend.write(&key4, value4).await.unwrap();

    backend.cache().run_pending_tasks().await;

    // Entry 4 should exist
    assert!(
        backend.read(&key4).await.unwrap().is_some(),
        "Entry 4 should exist after insert"
    );

    // Count remaining entries (should be 3)
    assert_eq!(
        backend.entry_count(),
        3,
        "Should have exactly 3 entries after eviction"
    );
}

#[tokio::test]
async fn test_max_bytes_large_vs_small_entries() {
    let small_value = make_value(50);
    let large_value = make_value(200);

    let small_key = make_key(1);
    let large_key = make_key(2);

    let small_entry_size = entry_size(&small_key, &small_value);
    let large_entry_size = entry_size(&large_key, &large_value);

    // Capacity for 1 large entry plus a small buffer
    // This means 2 small entries fit, but adding a large entry triggers eviction
    let capacity = large_entry_size + small_entry_size / 2;

    let backend = MokaBackend::builder().max_bytes(capacity as u64).build();

    // Insert 2 small entries - both should fit (2 * small < capacity)
    backend.write(&make_key(1), make_value(50)).await.unwrap();
    backend.write(&make_key(2), make_value(50)).await.unwrap();

    backend.cache().run_pending_tasks().await;

    assert!(backend.read(&make_key(1)).await.unwrap().is_some());
    assert!(backend.read(&make_key(2)).await.unwrap().is_some());

    // Insert 1 large entry - should evict at least one small entry
    backend.write(&make_key(3), make_value(200)).await.unwrap();

    backend.cache().run_pending_tasks().await;

    // Large entry should exist
    assert!(
        backend.read(&make_key(3)).await.unwrap().is_some(),
        "Large entry should exist"
    );

    // At least one small entry should be evicted (total would exceed capacity)
    let small1 = backend.read(&make_key(1)).await.unwrap().is_some();
    let small2 = backend.read(&make_key(2)).await.unwrap().is_some();

    assert!(
        !small1 || !small2,
        "At least one small entry should be evicted to make room for large entry"
    );
}

#[tokio::test]
async fn test_max_bytes_key_size_matters() {
    // Create keys with very different sizes
    let small_key = CacheKey::new("s", 0, vec![KeyPart::new("a", Some("1"))]);

    // Large key with strings > 23 bytes (SmolStr inline threshold)
    // Each string exceeding 23 bytes adds (len - 23) bytes of heap overhead
    let large_key = CacheKey::new(
        "this_prefix_is_longer_than_23_bytes_for_sure",
        0,
        vec![KeyPart::new(
            "this_key_name_exceeds_inline_storage",
            Some("this_value_also_exceeds_the_limit"),
        )],
    );

    let small_key_size = small_key.memory_size();
    let large_key_size = large_key.memory_size();

    // Large key should be bigger due to heap allocations
    // Small key: all strings inline (0 heap overhead)
    // Large key: 44 + 36 + 33 = 113 chars, heap overhead = (44-23) + (36-23) + (33-23) = 44 bytes
    assert!(
        large_key_size > small_key_size,
        "Large key ({}) should be bigger than small key ({})",
        large_key_size,
        small_key_size
    );

    let value = make_value(10);

    let small_entry = entry_size(&small_key, &value);
    let large_entry = entry_size(&large_key, &value);

    // Set capacity to fit both entries
    let capacity = (small_entry + large_entry) as u64;
    let backend = MokaBackend::builder().max_bytes(capacity).build();

    // Insert large key
    backend.write(&large_key, value.clone()).await.unwrap();
    backend.cache().run_pending_tasks().await;

    assert!(backend.read(&large_key).await.unwrap().is_some());

    // Insert small key - should fit alongside large
    backend.write(&small_key, value.clone()).await.unwrap();
    backend.cache().run_pending_tasks().await;

    // Both should exist (total == capacity)
    let have_large = backend.read(&large_key).await.unwrap().is_some();
    let have_small = backend.read(&small_key).await.unwrap().is_some();

    assert!(
        have_large && have_small,
        "Both entries should fit: large={}, small={}, capacity={}",
        large_entry,
        small_entry,
        capacity
    );

    // Now insert another small entry - should trigger eviction
    let another_small_key = CacheKey::new("t", 0, vec![KeyPart::new("b", Some("2"))]);
    backend
        .write(&another_small_key, value.clone())
        .await
        .unwrap();
    backend.cache().run_pending_tasks().await;

    // New small entry should exist
    assert!(
        backend.read(&another_small_key).await.unwrap().is_some(),
        "Third entry should exist after eviction"
    );

    // Large entry should be evicted (LRU: it was inserted first)
    let have_large_after = backend.read(&large_key).await.unwrap().is_some();
    assert!(
        !have_large_after,
        "Large entry should be evicted (inserted first, LRU)"
    );
}

#[tokio::test]
async fn test_memory_size_values() {
    // Test that memory_size returns expected ranges
    let key = make_key(42);
    let value = make_value(100);

    let key_size = key.memory_size();
    let value_size = value.memory_size();

    // Print actual sizes for debugging
    println!("Key memory_size: {} bytes", key_size);
    println!("Value memory_size: {} bytes", value_size);
    println!("Total entry size: {} bytes", key_size + value_size);

    // Key structure:
    // - Arc control block: 16 bytes
    // - CacheKeyInner struct: ~60-80 bytes
    // - Vec<KeyPart> elements: ~56 bytes each
    // - Heap strings: 0 (all inline for small keys)
    assert!(
        key_size >= 100,
        "Key should be at least 100 bytes, got {}",
        key_size
    );
    assert!(
        key_size <= 300,
        "Key should be at most 300 bytes, got {}",
        key_size
    );

    // Value structure:
    // - CacheValue struct: ~64 bytes (Bytes + 2x Option<DateTime>)
    // - Data: 100 bytes
    assert!(
        value_size >= 100 + 50,
        "Value should be at least 150 bytes, got {}",
        value_size
    );
    assert!(
        value_size <= 100 + 150,
        "Value should be at most 250 bytes, got {}",
        value_size
    );
}

#[tokio::test]
async fn test_entry_count_max_entries_vs_max_bytes() {
    let key = make_key(1);
    let value = make_value(200);
    let entry_size = entry_size(&key, &value);

    // Create two backends:
    // 1. Entry-based: exactly 5 entries
    // 2. Byte-based: capacity for exactly 5 entries
    let entry_backend = MokaBackend::builder().max_entries(5).build();
    let byte_backend = MokaBackend::builder()
        .max_bytes((entry_size * 5) as u64)
        .build();

    // Insert 5 entries into both
    for i in 1..=5 {
        let key = make_key(i);
        let value = make_value(200);
        entry_backend.write(&key, value.clone()).await.unwrap();
        byte_backend.write(&key, value).await.unwrap();
    }

    entry_backend.cache().run_pending_tasks().await;
    byte_backend.cache().run_pending_tasks().await;

    // Both should have all 5
    let mut entry_count = 0;
    let mut byte_count = 0;
    for i in 1..=5 {
        let key = make_key(i);
        if entry_backend.read(&key).await.unwrap().is_some() {
            entry_count += 1;
        }
        if byte_backend.read(&key).await.unwrap().is_some() {
            byte_count += 1;
        }
    }

    assert_eq!(entry_count, 5, "Entry-based should have 5 entries");
    assert_eq!(byte_count, 5, "Byte-based should have 5 entries");

    // Insert 6th entry - both should evict
    let key6 = make_key(6);
    let value6 = make_value(200);
    entry_backend.write(&key6, value6.clone()).await.unwrap();
    byte_backend.write(&key6, value6).await.unwrap();

    entry_backend.cache().run_pending_tasks().await;
    byte_backend.cache().run_pending_tasks().await;

    // Recount
    entry_count = 0;
    byte_count = 0;
    for i in 1..=6 {
        let key = make_key(i);
        if entry_backend.read(&key).await.unwrap().is_some() {
            entry_count += 1;
        }
        if byte_backend.read(&key).await.unwrap().is_some() {
            byte_count += 1;
        }
    }

    assert_eq!(entry_count, 5, "Entry-based should still have 5 entries");
    assert_eq!(byte_count, 5, "Byte-based should still have 5 entries");
}

#[tokio::test]
async fn test_tiny_lfu_admission_policy_with_entry_capacity() {
    // TinyLFU is the default for entry-based capacity
    // TinyLFU uses admission policy: unpopular new keys may be rejected
    let backend = MokaBackend::builder().max_entries(3).build();

    // Insert 3 entries
    for i in 1..=3 {
        backend.write(&make_key(i), make_value(100)).await.unwrap();
    }
    backend.cache().run_pending_tasks().await;

    // All 3 should be present
    for i in 1..=3 {
        assert!(
            backend.read(&make_key(i)).await.unwrap().is_some(),
            "Entry {} should exist",
            i
        );
    }

    // Access entries 2 and 3 to make them "hot" (increase frequency)
    for _ in 0..5 {
        backend.read(&make_key(2)).await.unwrap();
        backend.read(&make_key(3)).await.unwrap();
    }
    backend.cache().run_pending_tasks().await;

    // Try to insert 4th entry (unpopular - never seen before)
    // With TinyLFU, this may be rejected by the admission policy
    backend.write(&make_key(4), make_value(100)).await.unwrap();
    backend.cache().run_pending_tasks().await;

    // Hot entries (2, 3) should still exist
    assert!(
        backend.read(&make_key(2)).await.unwrap().is_some(),
        "Hot entry 2 should exist"
    );
    assert!(
        backend.read(&make_key(3)).await.unwrap().is_some(),
        "Hot entry 3 should exist"
    );

    // Count total entries - should be at most 3
    assert!(backend.entry_count() <= 3, "Should have at most 3 entries");
}

#[tokio::test]
async fn test_explicit_lru_policy_with_entry_capacity() {
    // Override default TinyLFU with explicit LRU, also test label()
    let backend = MokaBackend::builder()
        .label("test-lru")
        .max_entries(3)
        .eviction_policy(EvictionPolicy::lru())
        .build();

    // Insert 3 entries
    for i in 1..=3 {
        backend.write(&make_key(i), make_value(100)).await.unwrap();
    }
    backend.cache().run_pending_tasks().await;

    // All 3 should be present
    for i in 1..=3 {
        assert!(
            backend.read(&make_key(i)).await.unwrap().is_some(),
            "Entry {} should exist",
            i
        );
    }

    // Insert 4th entry - triggers eviction
    backend.write(&make_key(4), make_value(100)).await.unwrap();
    backend.cache().run_pending_tasks().await;

    // Entry 4 should exist
    assert!(
        backend.read(&make_key(4)).await.unwrap().is_some(),
        "Entry 4 should exist"
    );

    // With LRU, entry 1 should be evicted (oldest/least recently used)
    // Note: exact eviction order depends on moka internals
    assert_eq!(
        backend.entry_count(),
        3,
        "Should have exactly 3 entries after eviction"
    );
}

#[tokio::test]
async fn test_explicit_tiny_lfu_policy_with_byte_capacity() {
    let key = make_key(1);
    let value = make_value(100);
    let single_entry_size = entry_size(&key, &value);

    // Override default LRU with explicit TinyLFU for byte-based capacity
    let backend = MokaBackend::builder()
        .max_bytes((single_entry_size * 3) as u64)
        .eviction_policy(EvictionPolicy::tiny_lfu())
        .build();

    // Insert 3 entries
    for i in 1..=3 {
        backend.write(&make_key(i), make_value(100)).await.unwrap();
    }
    backend.cache().run_pending_tasks().await;

    // All 3 should be present
    for i in 1..=3 {
        assert!(
            backend.read(&make_key(i)).await.unwrap().is_some(),
            "Entry {} should exist",
            i
        );
    }

    // Make entries 2 and 3 "hot" by accessing them
    for _ in 0..5 {
        backend.read(&make_key(2)).await.unwrap();
        backend.read(&make_key(3)).await.unwrap();
    }
    backend.cache().run_pending_tasks().await;

    // Insert 4th entry
    // With TinyLFU, new unpopular keys may be rejected by admission policy
    backend.write(&make_key(4), make_value(100)).await.unwrap();
    backend.cache().run_pending_tasks().await;

    // Count entries - TinyLFU may or may not admit the new entry
    // depending on frequency estimates
    // With TinyLFU, we should have at most 3 entries
    assert!(
        backend.entry_count() <= 3,
        "Should have at most 3 entries with TinyLFU"
    );
}
