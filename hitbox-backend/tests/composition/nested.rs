//! Tests for nested CompositionBackend (composition of compositions).
//!
//! This tests that CompositionBackend can be composed with other CompositionBackends,
//! creating multi-level cache hierarchies.

use chrono::Utc;
use std::sync::Arc;

use hitbox_backend::{CacheBackend, CompositionBackend, SyncBackend};
use hitbox_core::{
    BoxContext, CacheContext, CacheKey, CacheValue, CacheableResponse, EntityPolicyConfig, Offload,
    Predicate,
};
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use std::future::Future;

#[cfg(feature = "rkyv_format")]
use rkyv::{Archive, Serialize as RkyvSerialize};

use crate::common::TestBackend;

/// Test offload that spawns tasks with tokio::spawn
#[derive(Clone, Debug)]
struct TestOffload;

impl Offload<'static> for TestOffload {
    fn spawn<F>(&self, _kind: impl Into<SmolStr>, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(future);
    }

    fn spawn_with_key<F>(&self, _key: CacheKey, kind: impl Into<SmolStr>, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.spawn(kind, future);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "rkyv_format",
    derive(Archive, RkyvSerialize, rkyv::Deserialize)
)]
pub(super) struct TestValue {
    pub data: String,
}

impl CacheableResponse for TestValue {
    type Cached = Self;
    type Subject = Self;
    type IntoCachedFuture = std::future::Ready<hitbox_core::CachePolicy<Self::Cached, Self>>;
    type FromCachedFuture = std::future::Ready<Self>;

    async fn cache_policy<P: Predicate<Subject = Self::Subject> + Send + Sync>(
        self,
        _predicate: P,
        _config: &EntityPolicyConfig,
    ) -> hitbox_core::ResponseCachePolicy<Self> {
        unimplemented!()
    }

    fn into_cached(self) -> Self::IntoCachedFuture {
        unimplemented!()
    }

    fn from_cached(_cached: Self::Cached) -> Self::FromCachedFuture {
        unimplemented!()
    }
}

// =============================================================================
// Static Dispatch Tests (Concrete Types)
// =============================================================================

#[tokio::test]
async fn test_nested_composition_static_dispatch() {
    // Create a 3-level cache hierarchy:
    // Level 1 (fastest): L1
    // Level 2 (medium):  L2
    // Level 3 (slowest): L3
    //
    // Structure: CompositionBackend(L1, CompositionBackend(L2, L3))

    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let l3 = TestBackend::new();

    // Create inner composition: L2 + L3
    let l2_l3 = CompositionBackend::new(l2.clone(), l3.clone(), TestOffload);

    // Create outer composition: L1 + (L2 + L3)
    let cache = CompositionBackend::new(l1.clone(), l2_l3, TestOffload);

    let key = CacheKey::from_str("test", "key1");
    let value = CacheValue::new(
        TestValue {
            data: "test_value".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Write through nested composition - should populate all 3 levels
    let mut ctx: BoxContext = CacheContext::default().boxed();
    cache
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    // Verify all 3 levels have the data
    assert!(l1.has(&key), "L1 should have the value");
    assert!(l2.has(&key), "L2 should have the value");
    assert!(l3.has(&key), "L3 should have the value");

    // Read should return the value (from L1)
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(
        *result.unwrap().data(),
        TestValue {
            data: "test_value".to_string()
        }
    );
}

// =============================================================================
// Deep Nested Refill Tests - Shared Test Logic
// =============================================================================

/// Shared test logic for 3-level refill with Always policy
async fn run_refill_3_levels_test<B: CacheBackend + Send + Sync>(
    cache: B,
    l1: &TestBackend,
    l2: &TestBackend,
    l3: &TestBackend,
    key_suffix: &str,
) {
    let key = CacheKey::from_str("test", &format!("deep_refill_{}", key_suffix));
    let value = CacheValue::new(
        TestValue {
            data: "from_l3".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Step 1: Write through composition (populates all 3 levels in correct format)
    let mut ctx: BoxContext = CacheContext::default().boxed();
    cache
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    // Verify all levels have data after write
    assert!(l1.has(&key), "L1 should have data after write");
    assert!(l2.has(&key), "L2 should have data after write");
    assert!(l3.has(&key), "L3 should have data after write");

    // Step 2: Clear L1 and L2 to simulate miss scenario
    l1.clear();
    l2.clear();
    assert!(!l1.has(&key), "L1 should be empty after clear");
    assert!(!l2.has(&key), "L2 should be empty after clear");

    // Step 3: Read through composition (should hit L3)
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    let cached_value = result.expect("Should get value from L3");
    assert_eq!(cached_value.data().data, "from_l3");

    // Step 4: Simulate CacheFuture calling set() with the wrapped context
    cache
        .set::<TestValue>(&key, &cached_value, &mut ctx)
        .await
        .unwrap();

    // Step 5: Verify L1 and L2 are refilled
    assert!(l1.has(&key), "L1 should be refilled after set()");
    assert!(l2.has(&key), "L2 should be refilled after set()");
    assert!(l3.has(&key), "L3 should still have data");

    // Step 6: Verify data can be read from L1 now (proving refill worked)
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().data().data, "from_l3");
}

/// Shared test logic for 4-level refill with Always policy
async fn run_refill_4_levels_test<B: CacheBackend + Send + Sync>(
    cache: B,
    l1: &TestBackend,
    l2: &TestBackend,
    l3: &TestBackend,
    l4: &TestBackend,
    key_suffix: &str,
) {
    let key = CacheKey::from_str("test", &format!("very_deep_refill_{}", key_suffix));
    let value = CacheValue::new(
        TestValue {
            data: "from_l4".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Write through composition
    let mut ctx: BoxContext = CacheContext::default().boxed();
    cache
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    // Clear L1, L2, L3 to simulate miss scenario (only L4 has data)
    l1.clear();
    l2.clear();
    l3.clear();

    // Read through composition (should hit L4)
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    let cached_value = result.expect("Should get value from L4");
    assert_eq!(cached_value.data().data, "from_l4");

    // Simulate CacheFuture calling set() to trigger refill cascade
    cache
        .set::<TestValue>(&key, &cached_value, &mut ctx)
        .await
        .unwrap();

    // Verify all levels are refilled
    assert!(l1.has(&key), "L1 should be refilled");
    assert!(l2.has(&key), "L2 should be refilled");
    assert!(l3.has(&key), "L3 should be refilled");
    assert!(l4.has(&key), "L4 should still have data");
}

/// Shared test logic for Never policy preventing refill
async fn run_no_refill_never_policy_test<B: CacheBackend + Send + Sync>(
    cache: B,
    l1: &TestBackend,
    l2: &TestBackend,
    _l3: &TestBackend,
    key_suffix: &str,
) {
    use hitbox_core::ReadMode;

    let key = CacheKey::from_str("test", &format!("no_refill_{}", key_suffix));
    let value = CacheValue::new(
        TestValue {
            data: "from_l3".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Write through composition
    let mut ctx: BoxContext = CacheContext::default().boxed();
    cache
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    // Clear L1 and L2
    l1.clear();
    l2.clear();

    // Read through composition (should hit L3)
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some(), "Should get value from L3");

    // With Never policy, read_mode should NOT be Refill
    assert_eq!(
        ctx.read_mode(),
        ReadMode::Direct,
        "With Never policy, read_mode should be Direct (not Refill)"
    );

    // L1 and L2 should NOT be refilled
    assert!(!l1.has(&key), "L1 should NOT be refilled with Never policy");
    assert!(!l2.has(&key), "L2 should NOT be refilled with Never policy");
}

/// Shared test logic for Never policy skipping refill even with Refill mode
async fn run_never_policy_skips_refill_test<B: CacheBackend + Send + Sync>(
    cache: B,
    l1: &TestBackend,
    l2: &TestBackend,
    _l3: &TestBackend,
    key_suffix: &str,
) {
    use hitbox_core::ReadMode;

    let key = CacheKey::from_str("test", &format!("no_refill_forced_{}", key_suffix));
    let value = CacheValue::new(
        TestValue {
            data: "from_l3".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Write through composition
    let mut ctx: BoxContext = CacheContext::default().boxed();
    cache
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    // Clear L1 and L2
    l1.clear();
    l2.clear();

    // Read through composition (should hit L3)
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    let cached_value = result.expect("Should get value from L3");

    // Manually set Refill mode to simulate edge case
    ctx.set_read_mode(ReadMode::Refill);

    // Call set() with Refill mode - should still NOT refill due to Never policy
    cache
        .set::<TestValue>(&key, &cached_value, &mut ctx)
        .await
        .unwrap();

    // L1 and L2 should NOT be refilled (Never policy overrides Refill mode)
    assert!(
        !l1.has(&key),
        "L1 should NOT be refilled - Never policy should prevent it"
    );
    assert!(
        !l2.has(&key),
        "L2 should NOT be refilled - Never policy should prevent it"
    );
}

// =============================================================================
// Deep Nested Refill Tests - Concrete Types (Static Dispatch)
// =============================================================================

#[tokio::test]
async fn test_deep_nested_refill_3_levels() {
    use hitbox_backend::composition::policy::RefillPolicy;

    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let l3 = TestBackend::new();

    let inner =
        CompositionBackend::new(l2.clone(), l3.clone(), TestOffload).refill(RefillPolicy::Always);
    let outer =
        CompositionBackend::new(l1.clone(), inner, TestOffload).refill(RefillPolicy::Always);

    run_refill_3_levels_test(outer, &l1, &l2, &l3, "static").await;
}

#[tokio::test]
async fn test_deep_nested_refill_4_levels() {
    use hitbox_backend::composition::policy::RefillPolicy;

    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let l3 = TestBackend::new();
    let l4 = TestBackend::new();

    let innermost =
        CompositionBackend::new(l3.clone(), l4.clone(), TestOffload).refill(RefillPolicy::Always);
    let middle =
        CompositionBackend::new(l2.clone(), innermost, TestOffload).refill(RefillPolicy::Always);
    let outer =
        CompositionBackend::new(l1.clone(), middle, TestOffload).refill(RefillPolicy::Always);

    run_refill_4_levels_test(outer, &l1, &l2, &l3, &l4, "static").await;
}

#[tokio::test]
async fn test_deep_nested_no_refill_with_never_policy() {
    use hitbox_backend::composition::policy::RefillPolicy;

    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let l3 = TestBackend::new();

    let inner =
        CompositionBackend::new(l2.clone(), l3.clone(), TestOffload).refill(RefillPolicy::Never);
    let outer = CompositionBackend::new(l1.clone(), inner, TestOffload).refill(RefillPolicy::Never);

    run_no_refill_never_policy_test(outer, &l1, &l2, &l3, "static").await;
}

#[tokio::test]
async fn test_never_policy_skips_refill_even_with_refill_mode() {
    use hitbox_backend::composition::policy::RefillPolicy;

    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let l3 = TestBackend::new();

    let inner =
        CompositionBackend::new(l2.clone(), l3.clone(), TestOffload).refill(RefillPolicy::Never);
    let outer = CompositionBackend::new(l1.clone(), inner, TestOffload).refill(RefillPolicy::Never);

    run_never_policy_skips_refill_test(outer, &l1, &l2, &l3, "static").await;
}

// =============================================================================
// Deep Nested Refill Tests - Arc<SyncBackend> (Dynamic Dispatch)
// =============================================================================

#[tokio::test]
async fn test_deep_nested_refill_3_levels_arc_sync() {
    use hitbox_backend::composition::policy::RefillPolicy;

    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let l3 = TestBackend::new();

    let l1_arc: Arc<SyncBackend> = Arc::new(l1.clone());
    let l2_arc: Arc<SyncBackend> = Arc::new(l2.clone());
    let l3_arc: Arc<SyncBackend> = Arc::new(l3.clone());

    let inner = CompositionBackend::new(l2_arc, l3_arc, TestOffload).refill(RefillPolicy::Always);
    let outer = CompositionBackend::new(l1_arc, inner, TestOffload).refill(RefillPolicy::Always);

    run_refill_3_levels_test(outer, &l1, &l2, &l3, "arc_sync").await;
}

#[tokio::test]
async fn test_deep_nested_refill_4_levels_arc_sync() {
    use hitbox_backend::composition::policy::RefillPolicy;

    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let l3 = TestBackend::new();
    let l4 = TestBackend::new();

    let l1_arc: Arc<SyncBackend> = Arc::new(l1.clone());
    let l2_arc: Arc<SyncBackend> = Arc::new(l2.clone());
    let l3_arc: Arc<SyncBackend> = Arc::new(l3.clone());
    let l4_arc: Arc<SyncBackend> = Arc::new(l4.clone());

    let innermost =
        CompositionBackend::new(l3_arc, l4_arc, TestOffload).refill(RefillPolicy::Always);
    let middle =
        CompositionBackend::new(l2_arc, innermost, TestOffload).refill(RefillPolicy::Always);
    let outer = CompositionBackend::new(l1_arc, middle, TestOffload).refill(RefillPolicy::Always);

    run_refill_4_levels_test(outer, &l1, &l2, &l3, &l4, "arc_sync").await;
}

#[tokio::test]
async fn test_deep_nested_no_refill_with_never_policy_arc_sync() {
    use hitbox_backend::composition::policy::RefillPolicy;

    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let l3 = TestBackend::new();

    let l1_arc: Arc<SyncBackend> = Arc::new(l1.clone());
    let l2_arc: Arc<SyncBackend> = Arc::new(l2.clone());
    let l3_arc: Arc<SyncBackend> = Arc::new(l3.clone());

    let inner = CompositionBackend::new(l2_arc, l3_arc, TestOffload).refill(RefillPolicy::Never);
    let outer = CompositionBackend::new(l1_arc, inner, TestOffload).refill(RefillPolicy::Never);

    run_no_refill_never_policy_test(outer, &l1, &l2, &l3, "arc_sync").await;
}

#[tokio::test]
async fn test_never_policy_skips_refill_even_with_refill_mode_arc_sync() {
    use hitbox_backend::composition::policy::RefillPolicy;

    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let l3 = TestBackend::new();

    let l1_arc: Arc<SyncBackend> = Arc::new(l1.clone());
    let l2_arc: Arc<SyncBackend> = Arc::new(l2.clone());
    let l3_arc: Arc<SyncBackend> = Arc::new(l3.clone());

    let inner = CompositionBackend::new(l2_arc, l3_arc, TestOffload).refill(RefillPolicy::Never);
    let outer = CompositionBackend::new(l1_arc, inner, TestOffload).refill(RefillPolicy::Never);

    run_never_policy_skips_refill_test(outer, &l1, &l2, &l3, "arc_sync").await;
}

// =============================================================================
// Dynamic Dispatch Tests (Trait Objects)
// =============================================================================

#[tokio::test]
async fn test_nested_composition_dynamic_dispatch() {
    // Test nested composition with Arc<SyncBackend>

    let l1: Arc<SyncBackend> = Arc::new(TestBackend::new());
    let l2: Arc<SyncBackend> = Arc::new(TestBackend::new());
    let l3: Arc<SyncBackend> = Arc::new(TestBackend::new());

    // Create inner composition as trait object
    let l2_l3: Arc<SyncBackend> = Arc::new(CompositionBackend::new(l2, l3, TestOffload));

    // Create outer composition with trait object
    let cache = CompositionBackend::new(l1, l2_l3, TestOffload);

    let key = CacheKey::from_str("test", "dyn_key");
    let value = CacheValue::new(
        TestValue {
            data: "dynamic_value".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Write and read through dynamic dispatch
    let mut ctx: BoxContext = CacheContext::default().boxed();
    cache
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(
        *result.unwrap().data(),
        TestValue {
            data: "dynamic_value".to_string()
        }
    );
}

#[tokio::test]
async fn test_nested_composition_dynamic_as_trait_object() {
    // Test that the nested composition itself can be used as a trait object

    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let l3 = TestBackend::new();

    let key = CacheKey::from_str("test", "nested_trait");
    let value = CacheValue::new(
        TestValue {
            data: "trait_object_value".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Create nested composition
    let l2_l3 = CompositionBackend::new(l2, l3, TestOffload);
    let nested = CompositionBackend::new(l1, l2_l3, TestOffload);

    // Use the entire nested composition as a trait object
    let backend: Arc<SyncBackend> = Arc::new(nested);

    // Operations through trait object
    let mut ctx: BoxContext = CacheContext::default().boxed();
    backend
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = backend.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(
        *result.unwrap().data(),
        TestValue {
            data: "trait_object_value".to_string()
        }
    );
}

// =============================================================================
// TTL/Stale Metadata Tests - Shared Test Logic
// =============================================================================

/// Helper struct to hold backends for TTL/stale testing with inspection capability.
struct TtlTestBackends {
    l1: TestBackend,
    l2: TestBackend,
}

impl TtlTestBackends {
    fn two_level() -> Self {
        Self {
            l1: TestBackend::new(),
            l2: TestBackend::new(),
        }
    }
}

/// Test TTL preservation through write/read cycle.
async fn run_ttl_preserved_test<B: CacheBackend + Send + Sync>(
    cache: B,
    l1: &TestBackend,
    l2: &TestBackend,
) {
    let key = CacheKey::from_str("test", "ttl_key");
    let expire_time = Utc::now() + chrono::Duration::seconds(300);
    let value = CacheValue::new(
        TestValue {
            data: "ttl_test".to_string(),
        },
        Some(expire_time),
        None,
    );

    // Write through composition
    let mut ctx: BoxContext = CacheContext::default().boxed();
    cache
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    // Verify TTL is preserved in L1
    let l1_raw = l1.get_raw(&key).expect("L1 should have the value");
    assert!(l1_raw.expire().is_some(), "L1 should have expire time");
    assert_eq!(
        l1_raw.expire(),
        Some(expire_time),
        "L1 expire time should match"
    );

    // Verify TTL is preserved in L2
    let l2_raw = l2.get_raw(&key).expect("L2 should have the value");
    assert!(l2_raw.expire().is_some(), "L2 should have expire time");
    assert_eq!(
        l2_raw.expire(),
        Some(expire_time),
        "L2 expire time should match"
    );

    // Read back and verify TTL is preserved
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    let cache_value = result.unwrap();
    assert_eq!(
        cache_value.expire(),
        Some(expire_time),
        "Read expire time should match"
    );
}

/// Test stale preservation through write/read cycle.
async fn run_stale_preserved_test<B: CacheBackend + Send + Sync>(
    cache: B,
    l1: &TestBackend,
    l2: &TestBackend,
) {
    let key = CacheKey::from_str("test", "stale_key");
    let expire_time = Utc::now() + chrono::Duration::seconds(300);
    let stale_time = Utc::now() + chrono::Duration::seconds(60);
    let value = CacheValue::new(
        TestValue {
            data: "stale_test".to_string(),
        },
        Some(expire_time),
        Some(stale_time),
    );

    // Write through composition
    let mut ctx: BoxContext = CacheContext::default().boxed();
    cache
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    // Verify stale is preserved in L1
    let l1_raw = l1.get_raw(&key).expect("L1 should have the value");
    assert!(l1_raw.stale().is_some(), "L1 should have stale time");
    assert_eq!(
        l1_raw.stale(),
        Some(stale_time),
        "L1 stale time should match"
    );

    // Verify stale is preserved in L2
    let l2_raw = l2.get_raw(&key).expect("L2 should have the value");
    assert!(l2_raw.stale().is_some(), "L2 should have stale time");
    assert_eq!(
        l2_raw.stale(),
        Some(stale_time),
        "L2 stale time should match"
    );

    // Read back and verify stale is preserved
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    let cache_value = result.unwrap();
    assert_eq!(
        cache_value.stale(),
        Some(stale_time),
        "Read stale time should match"
    );
    assert_eq!(
        cache_value.expire(),
        Some(expire_time),
        "Read expire time should match"
    );
}

/// Test no TTL/stale preservation.
async fn run_no_ttl_no_stale_test<B: CacheBackend + Send + Sync>(cache: B, l1: &TestBackend) {
    let key = CacheKey::from_str("test", "no_ttl_key");
    let value = CacheValue::new(
        TestValue {
            data: "no_ttl_test".to_string(),
        },
        None,
        None,
    );

    // Write through composition
    let mut ctx: BoxContext = CacheContext::default().boxed();
    cache
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    // Verify no TTL/stale in L1
    let l1_raw = l1.get_raw(&key).expect("L1 should have the value");
    assert!(l1_raw.expire().is_none(), "L1 should have no expire time");
    assert!(l1_raw.stale().is_none(), "L1 should have no stale time");

    // Read back and verify
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    let cache_value = result.unwrap();
    assert!(
        cache_value.expire().is_none(),
        "Read should have no expire time"
    );
    assert!(
        cache_value.stale().is_none(),
        "Read should have no stale time"
    );
}

// =============================================================================
// TTL/Stale Tests - Concrete Types (TestBackend)
// =============================================================================

#[tokio::test]
async fn test_ttl_preserved_concrete() {
    let backends = TtlTestBackends::two_level();
    let cache = CompositionBackend::new(backends.l1.clone(), backends.l2.clone(), TestOffload);
    run_ttl_preserved_test(cache, &backends.l1, &backends.l2).await;
}

#[tokio::test]
async fn test_stale_preserved_concrete() {
    let backends = TtlTestBackends::two_level();
    let cache = CompositionBackend::new(backends.l1.clone(), backends.l2.clone(), TestOffload);
    run_stale_preserved_test(cache, &backends.l1, &backends.l2).await;
}

#[tokio::test]
async fn test_no_ttl_no_stale_concrete() {
    let backends = TtlTestBackends::two_level();
    let cache = CompositionBackend::new(backends.l1.clone(), backends.l2.clone(), TestOffload);
    run_no_ttl_no_stale_test(cache, &backends.l1).await;
}

// =============================================================================
// TTL/Stale Tests - Arc<SyncBackend> (Dynamic Dispatch)
// =============================================================================

#[tokio::test]
async fn test_ttl_preserved_arc_sync() {
    let backends = TtlTestBackends::two_level();
    let l1: Arc<SyncBackend> = Arc::new(backends.l1.clone());
    let l2: Arc<SyncBackend> = Arc::new(backends.l2.clone());
    let cache = CompositionBackend::new(l1, l2, TestOffload);
    run_ttl_preserved_test(cache, &backends.l1, &backends.l2).await;
}

#[tokio::test]
async fn test_stale_preserved_arc_sync() {
    let backends = TtlTestBackends::two_level();
    let l1: Arc<SyncBackend> = Arc::new(backends.l1.clone());
    let l2: Arc<SyncBackend> = Arc::new(backends.l2.clone());
    let cache = CompositionBackend::new(l1, l2, TestOffload);
    run_stale_preserved_test(cache, &backends.l1, &backends.l2).await;
}

#[tokio::test]
async fn test_no_ttl_no_stale_arc_sync() {
    let backends = TtlTestBackends::two_level();
    let l1: Arc<SyncBackend> = Arc::new(backends.l1.clone());
    let l2: Arc<SyncBackend> = Arc::new(backends.l2.clone());
    let cache = CompositionBackend::new(l1, l2, TestOffload);
    run_no_ttl_no_stale_test(cache, &backends.l1).await;
}

#[tokio::test]
async fn test_nested_composition_delete_cascades() {
    // Test that delete operations cascade through nested compositions

    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let l3 = TestBackend::new();

    let key = CacheKey::from_str("test", "delete_key");
    let value = CacheValue::new(
        TestValue {
            data: "to_delete".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Populate all levels
    let mut ctx: BoxContext = CacheContext::default().boxed();
    l1.set::<TestValue>(&key, &value, &mut ctx).await.unwrap();
    let mut ctx: BoxContext = CacheContext::default().boxed();
    l2.set::<TestValue>(&key, &value, &mut ctx).await.unwrap();
    let mut ctx: BoxContext = CacheContext::default().boxed();
    l3.set::<TestValue>(&key, &value, &mut ctx).await.unwrap();

    // Verify all have the data
    assert!(l1.has(&key));
    assert!(l2.has(&key));
    assert!(l3.has(&key));

    // Create nested composition and delete
    let l2_l3 = CompositionBackend::new(l2.clone(), l3.clone(), TestOffload);
    let cache = CompositionBackend::new(l1.clone(), l2_l3, TestOffload);

    let mut ctx: BoxContext = CacheContext::default().boxed();
    cache.delete(&key, &mut ctx).await.unwrap();

    // Verify all levels no longer have the data
    assert!(!l1.has(&key), "L1 should be deleted");
    assert!(!l2.has(&key), "L2 should be deleted");
    assert!(!l3.has(&key), "L3 should be deleted");
}
