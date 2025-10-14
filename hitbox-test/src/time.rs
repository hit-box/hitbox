//! Mock time implementation for testing time-dependent caching behavior.
//!
//! This module provides a `MockTime` type that allows controlling the passage of time
//! in tests without actually waiting. This is particularly useful for testing cache
//! expiration, TTL, and stale cache mechanics.

mod cache_helpers;
mod mock_provider;

pub use cache_helpers::{clear_mock_time_provider, setup_mock_time_for_testing};
pub use mock_provider::MockTimeProvider;

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// A mock time provider that allows controlling the current time in tests.
///
/// `MockTime` maintains an internal offset from the real system time, allowing tests
/// to "advance" time without actually waiting. This is thread-safe and can be cloned
/// to share the same time state across multiple components.
///
/// # Examples
///
/// ```
/// use hitbox_test::time::MockTime;
/// use std::time::Duration;
///
/// let mock_time = MockTime::new();
/// let now1 = mock_time.now();
///
/// // Advance time by 5 seconds without actually sleeping
/// mock_time.advance(Duration::from_secs(5));
///
/// let now2 = mock_time.now();
/// assert!(now2 > now1);
/// ```
#[derive(Debug, Clone)]
pub struct MockTime {
    /// Shared state containing the time offset from real time
    state: Arc<Mutex<MockTimeState>>,
}

#[derive(Debug)]
struct MockTimeState {
    /// The offset to add to the real system time
    offset: Duration,
    /// The base system time when this MockTime was created
    base_time: SystemTime,
}

impl MockTime {
    /// Creates a new `MockTime` instance starting at the current system time.
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_test::time::MockTime;
    ///
    /// let mock_time = MockTime::new();
    /// ```
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockTimeState {
                offset: Duration::ZERO,
                base_time: SystemTime::now(),
            })),
        }
    }

    /// Creates a new `MockTime` instance starting at a specific time.
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_test::time::MockTime;
    /// use std::time::SystemTime;
    ///
    /// let specific_time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1000000);
    /// let mock_time = MockTime::with_time(specific_time);
    /// assert_eq!(mock_time.now(), specific_time);
    /// ```
    pub fn with_time(time: SystemTime) -> Self {
        Self {
            state: Arc::new(Mutex::new(MockTimeState {
                offset: Duration::ZERO,
                base_time: time,
            })),
        }
    }

    /// Returns the current mocked time.
    ///
    /// This includes any time advances that have been applied via `advance()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_test::time::MockTime;
    /// use std::time::Duration;
    ///
    /// let mock_time = MockTime::new();
    /// let now = mock_time.now();
    ///
    /// mock_time.advance(Duration::from_secs(10));
    /// let later = mock_time.now();
    ///
    /// assert!(later > now);
    /// ```
    pub fn now(&self) -> SystemTime {
        let state = self.state.lock().unwrap();
        state.base_time + state.offset
    }

    /// Advances the mocked time by the specified duration.
    ///
    /// This does not cause the thread to sleep; it merely updates the internal
    /// time offset so that subsequent calls to `now()` will return a later time.
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_test::time::MockTime;
    /// use std::time::Duration;
    ///
    /// let mock_time = MockTime::new();
    /// let before = mock_time.now();
    ///
    /// mock_time.advance(Duration::from_secs(3600)); // Advance by 1 hour
    ///
    /// let after = mock_time.now();
    /// assert_eq!(
    ///     after.duration_since(before).unwrap(),
    ///     Duration::from_secs(3600)
    /// );
    /// ```
    pub fn advance(&self, duration: Duration) {
        let mut state = self.state.lock().unwrap();
        state.offset += duration;
    }

    /// Advances the mocked time by the specified number of seconds.
    ///
    /// This is a convenience method that calls `advance()` with a duration
    /// created from the provided number of seconds.
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_test::time::MockTime;
    /// use std::time::Duration;
    ///
    /// let mock_time = MockTime::new();
    /// let before = mock_time.now();
    ///
    /// mock_time.advance_secs(30);
    ///
    /// let after = mock_time.now();
    /// assert_eq!(
    ///     after.duration_since(before).unwrap(),
    ///     Duration::from_secs(30)
    /// );
    /// ```
    pub fn advance_secs(&self, secs: u64) {
        self.advance(Duration::from_secs(secs));
    }

    /// Resets the mocked time back to the original base time.
    ///
    /// This removes any time advances that were applied via `advance()`.
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_test::time::MockTime;
    /// use std::time::Duration;
    ///
    /// let mock_time = MockTime::new();
    /// let original = mock_time.now();
    ///
    /// mock_time.advance(Duration::from_secs(100));
    /// mock_time.reset();
    ///
    /// let after_reset = mock_time.now();
    /// // Note: after_reset might be slightly later than original due to real time passing
    /// assert!(after_reset >= original);
    /// ```
    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        state.offset = Duration::ZERO;
        state.base_time = SystemTime::now();
    }

    /// Returns the total duration that time has been advanced.
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_test::time::MockTime;
    /// use std::time::Duration;
    ///
    /// let mock_time = MockTime::new();
    /// assert_eq!(mock_time.elapsed(), Duration::ZERO);
    ///
    /// mock_time.advance(Duration::from_secs(10));
    /// mock_time.advance(Duration::from_secs(5));
    ///
    /// assert_eq!(mock_time.elapsed(), Duration::from_secs(15));
    /// ```
    pub fn elapsed(&self) -> Duration {
        let state = self.state.lock().unwrap();
        state.offset
    }

    /// Sets the mocked time to a specific point in time.
    ///
    /// This replaces the base time and resets the offset to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_test::time::MockTime;
    /// use std::time::{SystemTime, Duration};
    ///
    /// let mock_time = MockTime::new();
    ///
    /// let specific_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1000000);
    /// mock_time.set_time(specific_time);
    ///
    /// assert_eq!(mock_time.now(), specific_time);
    /// ```
    pub fn set_time(&self, time: SystemTime) {
        let mut state = self.state.lock().unwrap();
        state.base_time = time;
        state.offset = Duration::ZERO;
    }
}

impl Default for MockTime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_mock_time_new() {
        let mock_time = MockTime::new();
        let now = SystemTime::now();
        let mock_now = mock_time.now();

        // Should be approximately equal (within a few milliseconds)
        let diff = mock_now
            .duration_since(now)
            .or_else(|_| now.duration_since(mock_now))
            .unwrap();
        assert!(diff < Duration::from_millis(100));
    }

    #[test]
    fn test_mock_time_with_specific_time() {
        let specific_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1000000);
        let mock_time = MockTime::with_time(specific_time);

        assert_eq!(mock_time.now(), specific_time);
    }

    #[test]
    fn test_advance_time() {
        let mock_time = MockTime::new();
        let before = mock_time.now();

        mock_time.advance(Duration::from_secs(10));

        let after = mock_time.now();
        assert_eq!(
            after.duration_since(before).unwrap(),
            Duration::from_secs(10)
        );
    }

    #[test]
    fn test_advance_secs() {
        let mock_time = MockTime::new();
        let before = mock_time.now();

        mock_time.advance_secs(30);

        let after = mock_time.now();
        assert_eq!(
            after.duration_since(before).unwrap(),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn test_multiple_advances() {
        let mock_time = MockTime::new();
        let start = mock_time.now();

        mock_time.advance(Duration::from_secs(5));
        mock_time.advance(Duration::from_secs(10));
        mock_time.advance(Duration::from_secs(15));

        let end = mock_time.now();
        assert_eq!(end.duration_since(start).unwrap(), Duration::from_secs(30));
    }

    #[test]
    fn test_elapsed() {
        let mock_time = MockTime::new();
        assert_eq!(mock_time.elapsed(), Duration::ZERO);

        mock_time.advance(Duration::from_secs(10));
        assert_eq!(mock_time.elapsed(), Duration::from_secs(10));

        mock_time.advance(Duration::from_secs(5));
        assert_eq!(mock_time.elapsed(), Duration::from_secs(15));
    }

    #[test]
    fn test_reset() {
        let mock_time = MockTime::new();

        mock_time.advance(Duration::from_secs(100));
        assert_eq!(mock_time.elapsed(), Duration::from_secs(100));

        mock_time.reset();
        assert_eq!(mock_time.elapsed(), Duration::ZERO);
    }

    #[test]
    fn test_set_time() {
        let mock_time = MockTime::new();

        let specific_time = SystemTime::UNIX_EPOCH + Duration::from_secs(2000000);
        mock_time.set_time(specific_time);

        assert_eq!(mock_time.now(), specific_time);
        assert_eq!(mock_time.elapsed(), Duration::ZERO);
    }

    #[test]
    fn test_clone_shares_state() {
        let mock_time1 = MockTime::new();
        let mock_time2 = mock_time1.clone();

        let before1 = mock_time1.now();
        let before2 = mock_time2.now();

        mock_time1.advance(Duration::from_secs(10));

        let after1 = mock_time1.now();
        let after2 = mock_time2.now();

        assert_eq!(after1, after2);
        assert_eq!(
            after1.duration_since(before1).unwrap(),
            Duration::from_secs(10)
        );
        assert_eq!(
            after2.duration_since(before2).unwrap(),
            Duration::from_secs(10)
        );
    }

    #[test]
    fn test_thread_safety() {
        let mock_time = MockTime::new();
        let mock_time_clone = mock_time.clone();

        let handle = thread::spawn(move || {
            mock_time_clone.advance(Duration::from_secs(5));
        });

        handle.join().unwrap();

        assert_eq!(mock_time.elapsed(), Duration::from_secs(5));
    }
}
