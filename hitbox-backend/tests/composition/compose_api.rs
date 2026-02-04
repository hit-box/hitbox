//! Tests for the Compose trait API.
//!
//! These tests demonstrate the fluent API for building CompositionBackend hierarchies.

use std::future::Future;

use chrono::Utc;
use hitbox_backend::composition::CompositionPolicy;
use hitbox_backend::composition::policy::{RaceReadPolicy, RefillPolicy};
use hitbox_backend::{CacheBackend, Compose};
use hitbox_core::{BoxContext, CacheContext, CacheKey, CacheValue, Offload};
use smol_str::SmolStr;

use crate::common::TestBackend;

// Reuse TestValue from nested tests
use super::nested::TestValue;

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

#[tokio::test]
async fn test_compose_trait_basic_usage() {
    // Basic usage: l1.compose(l2)
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();

    let cache = l1.clone().compose(l2.clone(), TestOffload);

    // Use from_slice to test that API path
    let key = CacheKey::from_slice(&[("test", Some("key1"))]);
    let value = CacheValue::new(
        TestValue {
            data: "compose_api".to_string(),
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

    // Read through composition
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(
        *result.unwrap().data(),
        TestValue {
            data: "compose_api".to_string()
        }
    );

    // Both layers should have the data
    assert!(l1.has(&key));
    assert!(l2.has(&key));
}

#[tokio::test]
async fn test_compose_with_custom_policy() {
    // Using compose_with to specify custom policies
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();

    let policy = CompositionPolicy::new()
        .read(RaceReadPolicy::new())
        .refill(RefillPolicy::Never);

    let cache = l1.clone().compose_with(l2.clone(), TestOffload, policy);

    let key = CacheKey::from_str("test", "custom_policy");
    let value = CacheValue::new(
        TestValue {
            data: "custom_value".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Populate only L2
    let mut ctx: BoxContext = CacheContext::default().boxed();
    l2.set::<TestValue>(&key, &value, &mut ctx).await.unwrap();

    // Read through composition (uses RaceReadPolicy)
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert!(result.is_some());
    assert_eq!(
        *result.unwrap().data(),
        TestValue {
            data: "custom_value".to_string()
        }
    );

    // With RefillPolicy::Never, L1 should NOT be populated
    assert!(!l1.has(&key));
}

#[tokio::test]
async fn test_compose_nested_3_levels() {
    // Create 3-level hierarchy using compose trait
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();
    let l3 = TestBackend::new();

    // Build: L1 + (L2 + L3)
    let cache = l1
        .clone()
        .compose(l2.clone().compose(l3.clone(), TestOffload), TestOffload);

    let key = CacheKey::from_str("test", "nested_compose");
    let value = CacheValue::new(
        TestValue {
            data: "nested_value".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Write cascades to all 3 levels
    let mut ctx: BoxContext = CacheContext::default().boxed();
    cache
        .set::<TestValue>(&key, &value, &mut ctx)
        .await
        .unwrap();

    // All levels should have the data
    assert!(l1.has(&key), "L1 should have the value");
    assert!(l2.has(&key), "L2 should have the value");
    assert!(l3.has(&key), "L3 should have the value");

    // Read returns the value
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert_eq!(
        *result.unwrap().data(),
        TestValue {
            data: "nested_value".to_string()
        }
    );
}

#[tokio::test]
async fn test_compose_with_builder_chaining() {
    // Combine compose with builder methods
    let l1 = TestBackend::new();
    let l2 = TestBackend::new();

    let cache = l1
        .clone()
        .compose(l2.clone(), TestOffload)
        .read(RaceReadPolicy::new())
        .refill(RefillPolicy::Never);

    let key = CacheKey::from_str("test", "chained");
    let value = CacheValue::new(
        TestValue {
            data: "chained_value".to_string(),
        },
        Some(Utc::now() + chrono::Duration::seconds(60)),
        None,
    );

    // Populate only L2
    let mut ctx: BoxContext = CacheContext::default().boxed();
    l2.set::<TestValue>(&key, &value, &mut ctx).await.unwrap();

    // Read through composition
    let mut ctx: BoxContext = CacheContext::default().boxed();
    let result = cache.get::<TestValue>(&key, &mut ctx).await.unwrap();
    assert_eq!(
        *result.unwrap().data(),
        TestValue {
            data: "chained_value".to_string()
        }
    );

    // With RefillPolicy::Never, L1 should not be populated
    assert!(!l1.has(&key));
}
