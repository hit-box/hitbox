//! Time provider abstraction for testing cache TTL and expiration logic.
//!
//! This module provides a `TimeProvider` trait that allows mocking time
//! for testing purposes. In production builds, `CacheValue` uses `Utc::now()`
//! directly for zero overhead.

use chrono::{DateTime, Utc};

/// Trait for providing current time, allowing for mocking in tests.
///
/// This trait abstracts time retrieval to enable testing of time-dependent
/// cache behavior (TTL, stale cache, expiration) without actually waiting.
///
/// In production builds, this trait is only used when the `test-helpers`
/// feature is enabled. Otherwise, `CacheValue` calls `Utc::now()` directly.
pub trait TimeProvider: Send + Sync {
    /// Returns the current time as a UTC DateTime.
    fn now(&self) -> DateTime<Utc>;
}
