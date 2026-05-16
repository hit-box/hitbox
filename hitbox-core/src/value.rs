//! Cached value types with expiration metadata.
//!
//! This module provides types for wrapping cached data with expiration
//! and staleness timestamps:
//!
//! - [`CacheValue`] - Cached data with creation time and expire/stale timestamps
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
//! let now = Utc::now();
//! let value = CacheValue::new(
//!     "cached data",
//!     now,
//!     Some(now + chrono::Duration::hours(1)),   // expires in 1 hour
//!     Some(now + chrono::Duration::minutes(5)), // stale in 5 minutes
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
use crate::tag::CacheTags;

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
    created: DateTime<Utc>,
    expire: Option<DateTime<Utc>>,
    stale: Option<DateTime<Utc>>,
    tags: Option<CacheTags>,
}

impl<T> CacheValue<T> {
    /// Creates a new cache value with the given data and timestamps.
    ///
    /// Sets the creation time to the current time automatically.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to cache
    /// * `expire` - When the data expires (becomes invalid)
    /// * `stale` - When the data becomes stale (should refresh in background)
    pub fn new(data: T, expire: Option<DateTime<Utc>>, stale: Option<DateTime<Utc>>) -> Self {
        CacheValue {
            data,
            created: Utc::now(),
            expire,
            stale,
            tags: None,
        }
    }

    /// Creates a cache value with an explicit creation timestamp.
    ///
    /// Use this when restoring a value from a backend where the creation
    /// time is known (e.g., deserialization from Redis or envelope format).
    ///
    /// # Arguments
    ///
    /// * `data` - The data to cache
    /// * `created` - When the entry was originally created
    /// * `expire` - When the data expires (becomes invalid)
    /// * `stale` - When the data becomes stale (should refresh in background)
    pub fn with_created(
        data: T,
        created: DateTime<Utc>,
        expire: Option<DateTime<Utc>>,
        stale: Option<DateTime<Utc>>,
    ) -> Self {
        CacheValue {
            data,
            created,
            expire,
            stale,
            tags: None,
        }
    }

    /// Attaches tags to this cache value (builder-style).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let value = CacheValue::new(data, expire, stale).with_tags(tags);
    /// ```
    pub fn with_tags(mut self, tags: CacheTags) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Returns a reference to the cached value's tags, if any.
    ///
    /// Returns `None` if no tag system was used when writing this entry,
    /// or `Some(&tags)` if tag extractors were configured (tags may still be empty).
    #[inline]
    pub fn tags(&self) -> Option<&CacheTags> {
        self.tags.as_ref()
    }

    /// Sets tags from an `Option<CacheTags>` (builder-style).
    ///
    /// Convenience wrapper for passing tags forward when reconstructing
    /// `CacheValue` from metadata. Use [`with_tags`] for direct attachment.
    ///
    /// [`with_tags`]: Self::with_tags
    pub fn with_optional_tags(mut self, tags: Option<CacheTags>) -> Self {
        self.tags = tags;
        self
    }

    /// Creates a new cache value using timestamps derived from an [`EntityPolicyConfig`].
    ///
    /// Sets the creation time to now and converts the config's TTL and stale TTL
    /// durations into absolute timestamps relative to the current time.
    pub fn from_config(data: T, config: &EntityPolicyConfig) -> Self {
        let now = Utc::now();
        Self::with_created(
            data,
            now,
            config.ttl.map(|d| now + d),
            config.stale_ttl.map(|d| now + d),
        )
    }

    /// Returns a reference to the cached data.
    #[inline]
    pub fn data(&self) -> &T {
        &self.data
    }

    /// Returns when the entry was created.
    #[inline]
    pub fn created(&self) -> DateTime<Utc> {
        self.created
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
        (
            CacheMeta::new(self.created, self.expire, self.stale, self.tags),
            self.data,
        )
    }

    /// Calculate remaining TTL from the expire time.
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
/// Contains the creation timestamp and expiration/staleness timestamps.
/// Useful for passing metadata around without copying the cached data.
pub struct CacheMeta {
    /// When the cached data was created.
    pub created: DateTime<Utc>,
    /// When the cached data expires and becomes invalid.
    pub expire: Option<DateTime<Utc>>,
    /// When the cached data becomes stale and should be refreshed.
    pub stale: Option<DateTime<Utc>>,
    /// Tags attached to the cached entry, if any.
    pub tags: Option<CacheTags>,
}

impl CacheMeta {
    /// Creates new cache metadata with the given timestamps and tags.
    pub fn new(
        created: DateTime<Utc>,
        expire: Option<DateTime<Utc>>,
        stale: Option<DateTime<Utc>>,
        tags: Option<CacheTags>,
    ) -> CacheMeta {
        CacheMeta {
            created,
            expire,
            stale,
            tags,
        }
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
