//! MockTimeProvider that implements hitbox_core::TimeProvider
//!
//! This allows MockTime to be used with CacheValue for testing TTL/stale mechanics.

use chrono::{DateTime, Utc};
use hitbox_core::TimeProvider;
use std::time::SystemTime;

use super::MockTime;

/// A time provider that wraps MockTime and implements hitbox_core::TimeProvider.
///
/// This allows using MockTime with `CacheValue` to test cache expiration, TTL,
/// and stale cache mechanics without actually waiting.
///
/// # Examples
///
/// ```
/// use hitbox_test::time::{MockTime, MockTimeProvider};
/// use hitbox_core::TimeProvider;
///
/// let mock_time = MockTime::new();
/// let provider = MockTimeProvider::new(mock_time.clone());
///
/// let now = provider.now();
///
/// // Advance time
/// mock_time.advance_secs(10);
///
/// let later = provider.now();
/// assert!(later > now);
/// ```
#[derive(Debug, Clone)]
pub struct MockTimeProvider {
    mock_time: MockTime,
}

impl MockTimeProvider {
    /// Creates a new MockTimeProvider wrapping the given MockTime.
    pub fn new(mock_time: MockTime) -> Self {
        Self { mock_time }
    }

    /// Creates a new MockTimeProvider with a fresh MockTime instance.
    pub fn with_new_time() -> Self {
        Self {
            mock_time: MockTime::new(),
        }
    }

    /// Get a reference to the underlying MockTime for advancing time.
    pub fn mock_time(&self) -> &MockTime {
        &self.mock_time
    }

    /// Convert a SystemTime to DateTime<Utc>.
    fn system_time_to_datetime(st: SystemTime) -> DateTime<Utc> {
        let duration_since_epoch = st
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("SystemTime before UNIX EPOCH");

        DateTime::from_timestamp(
            duration_since_epoch.as_secs() as i64,
            duration_since_epoch.subsec_nanos(),
        )
        .expect("Invalid timestamp")
    }
}

impl TimeProvider for MockTimeProvider {
    fn now(&self) -> DateTime<Utc> {
        Self::system_time_to_datetime(self.mock_time.now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_mock_time_provider_now() {
        let mock_time = MockTime::new();
        let provider = MockTimeProvider::new(mock_time.clone());

        let now1 = provider.now();
        mock_time.advance(Duration::from_secs(10));
        let now2 = provider.now();

        assert_eq!((now2 - now1).num_seconds(), 10);
    }

    #[test]
    fn test_mock_time_provider_with_new_time() {
        let provider = MockTimeProvider::with_new_time();
        let now = provider.now();
        let system_now = Utc::now();

        // Should be approximately equal
        let diff = (now - system_now).num_milliseconds().abs();
        assert!(diff < 1000);
    }

    #[test]
    fn test_mock_time_provider_advance() {
        let mock_time = MockTime::new();
        let provider = MockTimeProvider::new(mock_time.clone());

        let start = provider.now();
        mock_time.advance_secs(60);
        let end = provider.now();

        assert_eq!((end - start).num_seconds(), 60);
    }

    #[test]
    fn test_mock_time_provider_clone_shares_state() {
        let mock_time = MockTime::new();
        let provider1 = MockTimeProvider::new(mock_time.clone());
        let provider2 = provider1.clone();

        mock_time.advance_secs(30);

        let time1 = provider1.now();
        let time2 = provider2.now();

        assert_eq!(time1, time2);
    }
}
