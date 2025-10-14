//! Integration tests for MockTime with CacheValue.
//!
//! These tests verify that MockTime correctly integrates with CacheValue's
//! TTL and stale mechanics.
//!
//! These tests use the `#[serial]` attribute to ensure they run sequentially
//! because they share a global mock time provider.

use chrono::Utc;
use hitbox_core::{CacheState, CacheValue, TimeProvider};
use hitbox_test::time::{clear_mock_time_provider, setup_mock_time_for_testing};
use serial_test::serial;
use std::time::Duration;

// Simple test response type
#[derive(Debug, Clone, PartialEq, Eq)]
struct TestResponse {
    data: String,
}

// Implement CacheableResponse for TestResponse
#[async_trait::async_trait]
impl hitbox_core::CacheableResponse for TestResponse {
    type Cached = String;
    type Subject = TestResponse;

    async fn cache_policy<P>(
        self,
        _predicates: P,
        config: &hitbox_core::EntityPolicyConfig,
    ) -> hitbox_core::ResponseCachePolicy<Self>
    where
        P: hitbox_core::Predicate<Subject = Self::Subject> + Send + Sync,
    {
        let cached = self.data.clone();
        hitbox_core::CachePolicy::Cacheable(CacheValue::new(
            cached,
            config.ttl.map(|d| Utc::now() + d),
            config.stale_ttl.map(|d| Utc::now() + d),
        ))
    }

    async fn into_cached(self) -> hitbox_core::CachePolicy<Self::Cached, Self> {
        hitbox_core::CachePolicy::Cacheable(self.data)
    }

    async fn from_cached(cached: Self::Cached) -> Self {
        TestResponse { data: cached }
    }
}

#[tokio::test]
#[serial]
async fn test_mock_time_advances_correctly() {
    let (mock_time, provider) = setup_mock_time_for_testing();

    let start_time = provider.now();

    // Advance by 10 seconds
    mock_time.advance_secs(10);
    let after_10s = provider.now();

    let diff = (after_10s - start_time).num_seconds();
    assert_eq!(diff, 10, "Time should advance by 10 seconds");

    // Advance by 5 more seconds
    mock_time.advance_secs(5);
    let after_15s = provider.now();

    let total_diff = (after_15s - start_time).num_seconds();
    assert_eq!(total_diff, 15, "Time should advance by 15 seconds total");

    clear_mock_time_provider();
}

#[tokio::test]
#[serial]
async fn test_mock_time_with_cache_value_expiration() {
    let (mock_time, provider) = setup_mock_time_for_testing();

    // Create cache value that expires in 10 seconds from now
    let now = provider.now();
    let expire = now + Duration::from_secs(10);
    let cache_value = CacheValue::new("test data".to_string(), Some(expire), None);

    // Immediately - should be Actual
    let state = cache_value.clone().cache_state::<TestResponse>().await;
    assert!(
        matches!(state, CacheState::Actual(_)),
        "Cache should be Actual immediately after creation"
    );

    // After 5 seconds - still Actual (within TTL)
    mock_time.advance_secs(5);
    let state = cache_value.clone().cache_state::<TestResponse>().await;
    assert!(
        matches!(state, CacheState::Actual(_)),
        "Cache should still be Actual after 5 seconds (TTL is 10s)"
    );

    // After 11 seconds total - Expired (past TTL)
    mock_time.advance_secs(6);
    let state = cache_value.cache_state::<TestResponse>().await;
    assert!(
        matches!(state, CacheState::Expired(_)),
        "Cache should be Expired after 11 seconds (past 10s TTL)"
    );

    clear_mock_time_provider();
}

#[tokio::test]
#[serial]
async fn test_mock_time_reset() {
    let (mock_time, provider) = setup_mock_time_for_testing();

    let start_time = provider.now();

    // Advance time significantly
    mock_time.advance_secs(100);
    let advanced_time = provider.now();

    let elapsed = (advanced_time - start_time).num_seconds();
    assert_eq!(elapsed, 100, "Time should have advanced by 100 seconds");

    // Reset time to baseline
    mock_time.reset();
    let reset_time = provider.now();

    // After reset, time should be close to start (within a reasonable tolerance)
    let diff = (reset_time - start_time).num_milliseconds().abs();
    assert!(
        diff < 100,
        "Time should return to near start after reset (diff: {}ms)",
        diff
    );

    clear_mock_time_provider();
}

#[tokio::test]
#[serial]
async fn test_multiple_cache_values_share_mock_time() {
    let (mock_time, provider) = setup_mock_time_for_testing();

    let now = provider.now();

    // Create two cache values with different expiration times
    let expire1 = now + Duration::from_secs(10);
    let expire2 = now + Duration::from_secs(20);

    let cache1 = CacheValue::new("data1".to_string(), Some(expire1), None);
    let cache2 = CacheValue::new("data2".to_string(), Some(expire2), None);

    // Both should be Actual initially
    let state1 = cache1.clone().cache_state::<TestResponse>().await;
    let state2 = cache2.clone().cache_state::<TestResponse>().await;
    assert!(matches!(state1, CacheState::Actual(_)));
    assert!(matches!(state2, CacheState::Actual(_)));

    // Advance to 15 seconds - cache1 expired, cache2 still valid
    mock_time.advance_secs(15);
    let state1 = cache1.cache_state::<TestResponse>().await;
    let state2 = cache2.clone().cache_state::<TestResponse>().await;

    assert!(
        matches!(state1, CacheState::Expired(_)),
        "Cache1 should be expired after 15s (TTL was 10s)"
    );
    assert!(
        matches!(state2, CacheState::Actual(_)),
        "Cache2 should still be Actual after 15s (TTL is 20s)"
    );

    // Advance to 25 seconds total - both expired
    mock_time.advance_secs(10);
    let state2 = cache2.cache_state::<TestResponse>().await;
    assert!(
        matches!(state2, CacheState::Expired(_)),
        "Cache2 should be expired after 25s (TTL was 20s)"
    );

    clear_mock_time_provider();
}

#[tokio::test]
#[serial]
async fn test_cache_value_without_expiration() {
    let (_mock_time, _provider) = setup_mock_time_for_testing();

    // Create cache value with no expiration
    let cache_value = CacheValue::new("eternal data".to_string(), None, None);

    // Should always be Actual
    let state = cache_value.clone().cache_state::<TestResponse>().await;
    assert!(
        matches!(state, CacheState::Actual(_)),
        "Cache without expiration should always be Actual"
    );

    clear_mock_time_provider();
}

#[tokio::test]
#[serial]
async fn test_data_preserved_in_all_states() {
    let (mock_time, provider) = setup_mock_time_for_testing();

    let test_data = "important data".to_string();
    let now = provider.now();
    let stale = now + Duration::from_secs(5); // Stale after 5 seconds
    let expire = now + Duration::from_secs(10); // Expire after 10 seconds
    let cache_value = CacheValue::new(test_data.clone(), Some(expire), Some(stale));

    // Verify data in Actual state (T+0)
    let state = cache_value.clone().cache_state::<TestResponse>().await;
    match state {
        CacheState::Actual(response) => {
            assert_eq!(
                response.data, test_data,
                "Data should be preserved in Actual state"
            );
        }
        _ => panic!("Expected Actual state"),
    }

    // Verify data in Stale state (T+6, past stale time but within TTL)
    mock_time.advance_secs(6);
    let state = cache_value.clone().cache_state::<TestResponse>().await;
    match state {
        CacheState::Stale(response) => {
            assert_eq!(
                response.data, test_data,
                "Data should be preserved in Stale state"
            );
        }
        _ => panic!("Expected Stale state, but got: {:?}", state),
    }

    // Verify data in Expired state (T+11, past TTL)
    mock_time.advance_secs(5);
    let state = cache_value.cache_state::<TestResponse>().await;
    match state {
        CacheState::Expired(response) => {
            assert_eq!(
                response.data, test_data,
                "Data should be preserved in Expired state"
            );
        }
        _ => panic!("Expected Expired state"),
    }

    clear_mock_time_provider();
}
