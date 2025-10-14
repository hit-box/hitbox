//! Helper functions for testing cache behavior with mock time.

use super::{MockTime, MockTimeProvider};
use hitbox_core::set_mock_time_provider;

/// Sets up mock time for testing cache behavior.
///
/// This function creates a MockTime and MockTimeProvider, and sets it as the
/// global time provider for `CacheValue`. This enables testing TTL and stale
/// cache mechanics without actually waiting.
///
/// # Returns
///
/// Returns a tuple of (`MockTime`, `MockTimeProvider`) so you can advance time
/// and get the current mocked time in tests.
///
/// # Examples
///
/// ```
/// use hitbox_test::time::setup_mock_time_for_testing;
/// use hitbox_core::TimeProvider;
/// use std::time::Duration;
///
/// let (mock_time, provider) = setup_mock_time_for_testing();
///
/// // Get current time from the provider
/// let now = provider.now();
///
/// // Advance time in your tests
/// mock_time.advance(Duration::from_secs(30));
/// ```
///
/// # Cleanup
///
/// To cleanup after tests, use `clear_mock_time_provider()`.
pub fn setup_mock_time_for_testing() -> (MockTime, MockTimeProvider) {
    let mock_time = MockTime::new();
    let provider = MockTimeProvider::new(mock_time.clone());

    // Set as global time provider
    set_mock_time_provider(Some(Box::new(provider.clone())));

    (mock_time, provider)
}

/// Clears the mock time provider, restoring normal time behavior.
///
/// This should be called at the end of tests that use `setup_mock_time_for_testing()`.
///
/// # Examples
///
/// ```
/// use hitbox_test::time::{setup_mock_time_for_testing, clear_mock_time_provider};
///
/// let mock_time = setup_mock_time_for_testing();
/// // ... do testing ...
/// clear_mock_time_provider();
/// ```
pub fn clear_mock_time_provider() {
    set_mock_time_provider(None);
}
