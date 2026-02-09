//! Cached value types with expiration metadata.
//!
//! This module provides types for wrapping cached data with expiration
//! and staleness timestamps:
//!
//! - [`CacheValue`] - Cached data with optional expire and stale timestamps
//! - [`CacheMeta`] - Just the metadata without the data
//!
//! ## Expiration vs Staleness
//!
//! Cache entries have two time-based states:
//!
//! - **Stale** - The data is still usable but should be refreshed in the background
//! - **Expired** - The data is no longer valid and must be refreshed before use
//!
//! This allows implementing "stale-while-revalidate" caching patterns where
//! stale data is served immediately while fresh data is fetched asynchronously.
//!
//! ## Cache States
//!
//! The [`CacheValue::cache_state`] method evaluates timestamps and returns:
//!
//! - [`CacheState::Actual`] - Data is fresh (neither stale nor expired)
//! - [`CacheState::Stale`] - Data is stale but not expired
//! - [`CacheState::Expired`] - Data has expired
//!
//! ```ignore
//! use hitbox_core::value::CacheValue;
//! use chrono::Utc;
//!
//! let value = CacheValue::new(
//!     "cached data",
//!     Some(Utc::now() + chrono::Duration::hours(1)),  // expires in 1 hour
//!     Some(Utc::now() + chrono::Duration::minutes(5)), // stale in 5 minutes
//! );
//!
//! match value.cache_state() {
//!     CacheState::Actual(v) => println!("Fresh: {:?}", v.data()),
//!     CacheState::Stale(v) => println!("Stale, refresh in background"),
//!     CacheState::Expired(v) => println!("Expired, must refresh"),
//! }
//! ```

use chrono::{DateTime, Utc};
use std::mem::size_of;
use std::time::Duration;

use crate::Raw;
use crate::policy::EntityPolicyConfig;
use crate::response::CacheState;

/// A cached value with expiration metadata.
///
/// Wraps any data type `T` with optional timestamps for staleness and expiration.
/// This enables time-based cache invalidation and stale-while-revalidate patterns.
///
/// # Type Parameter
///
/// * `T` - The cached data type
///
/// # Example
///
/// ```
/// use hitbox_core::value::CacheValue;
/// use chrono::Utc;
/// use std::time::Duration;
///
/// // Create a cache value that expires in 1 hour
/// let expire_time = Utc::now() + chrono::Duration::hours(1);
/// let value = CacheValue::new("user_data", Some(expire_time), None);
///
/// // Access data via getter
/// assert_eq!(value.data(), &"user_data");
///
/// // Check remaining TTL
/// if let Some(ttl) = value.ttl() {
///     println!("Expires in {} seconds", ttl.as_secs());
/// }
///
/// // Extract the data
/// let data = value.into_inner();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheValue<T> {
    data: T,
    expire: Option<DateTime<Utc>>,
    stale: Option<DateTime<Utc>>,
}

impl<T> CacheValue<T> {
    /// Creates a new cache value with the given data and timestamps.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to cache
    /// * `expire` - When the data expires (becomes invalid)
    /// * `stale` - When the data becomes stale (should refresh in background)
    pub fn new(data: T, expire: Option<DateTime<Utc>>, stale: Option<DateTime<Utc>>) -> Self {
        CacheValue {
            data,
            expire,
            stale,
        }
    }

    /// Creates a new cache value using timestamps derived from an [`EntityPolicyConfig`].
    ///
    /// Converts the config's TTL and stale TTL durations into absolute timestamps
    /// relative to the current time.
    pub fn from_config(data: T, config: &EntityPolicyConfig) -> Self {
        Self::new(
            data,
            config.ttl.map(|d| Utc::now() + d),
            config.stale_ttl.map(|d| Utc::now() + d),
        )
    }

    /// Returns a reference to the cached data.
    #[inline]
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Returns when the data expires (becomes invalid).
    #[inline]
    pub fn expire(&self) -> Option<DateTime<Utc>> {
        self.expire
    }

    /// Returns when the data becomes stale (should refresh in background).
    #[inline]
    pub fn stale(&self) -> Option<DateTime<Utc>> {
        self.stale
    }

    /// Consumes the cache value and returns the inner data.
    ///
    /// Discards the expiration metadata.
    pub fn into_inner(self) -> T {
        self.data
    }

    /// Consumes the cache value and returns metadata and data separately.
    ///
    /// Useful when you need to inspect or modify the metadata independently.
    pub fn into_parts(self) -> (CacheMeta, T) {
        (CacheMeta::new(self.expire, self.stale), self.data)
    }

    /// Calculate TTL (time-to-live) from the expire time.
    ///
    /// Returns `Some(Duration)` if there's a valid expire time in the future,
    /// or `None` if there's no expire time or it's already expired.
    pub fn ttl(&self) -> Option<Duration> {
        self.expire.and_then(|expire| {
            let duration = expire.signed_duration_since(Utc::now());
            if duration.num_seconds() > 0 {
                Some(Duration::from_secs(duration.num_seconds() as u64))
            } else {
                None
            }
        })
    }
}

impl<T> CacheValue<T> {
    /// Check the cache state based on expire/stale timestamps.
    ///
    /// Returns `CacheState<CacheValue<T>>` preserving the original value with metadata.
    /// This is a sync operation - just checks timestamps, no conversion.
    ///
    /// The caller is responsible for converting to Response via `from_cached()` when needed.
    pub fn cache_state(self) -> CacheState<Self> {
        let now = Utc::now();
        if let Some(expire) = self.expire
            && expire <= now
        {
            CacheState::Expired(self)
        } else if let Some(stale) = self.stale
            && stale <= now
        {
            CacheState::Stale(self)
        } else {
            CacheState::Actual(self)
        }
    }
}

/// Cache expiration metadata without the data.
///
/// Contains just the staleness and expiration timestamps. Useful for
/// passing metadata around without copying the cached data.
///
/// # Fields
///
/// * `expire` - When the data expires (becomes invalid)
/// * `stale` - When the data becomes stale (should refresh in background)
pub struct CacheMeta {
    /// When the cached data expires and becomes invalid.
    pub expire: Option<DateTime<Utc>>,
    /// When the cached data becomes stale and should be refreshed.
    pub stale: Option<DateTime<Utc>>,
}

impl CacheMeta {
    /// Creates new cache metadata with the given timestamps.
    pub fn new(expire: Option<DateTime<Utc>>, stale: Option<DateTime<Utc>>) -> CacheMeta {
        CacheMeta { expire, stale }
    }
}

impl CacheValue<Raw> {
    /// Returns the estimated memory usage of this cache value in bytes.
    ///
    /// This includes:
    /// - Fixed struct overhead (CacheValue fields)
    /// - The serialized data bytes
    pub fn memory_size(&self) -> usize {
        // Fixed overhead: CacheValue struct (data pointer + metadata)
        let fixed_overhead = size_of::<Self>();

        // Variable content: the actual byte data
        let content = self.data.len();

        fixed_overhead + content
    }
}
