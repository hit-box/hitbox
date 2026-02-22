//! Cache policy types and configuration.
//!
//! This module provides types for representing cache decisions and
//! configuring caching behavior:
//!
//! - [`CachePolicy`] - Result of a cache decision (cacheable or not)
//! - [`EntityPolicyConfig`] - TTL configuration for cached entities
//! - [`PolicyConfig`] - Cache policy: enabled with settings or disabled
//!
//! ## Cache Policy
//!
//! [`CachePolicy`] represents the outcome of determining whether something
//! should be cached. It's a two-variant enum that preserves type information
//! for both cacheable and non-cacheable cases.
//!
//! ## Configuration
//!
//! [`EntityPolicyConfig`] provides TTL (time-to-live) settings for cached
//! entries, supporting both expiration and staleness timeouts for
//! stale-while-revalidate patterns.
//!
//! [`PolicyConfig`] controls overall cache behavior per endpoint: TTL,
//! stale windows, concurrency limits, and stale-while-revalidate policy.

use std::num::NonZeroU8;
use std::time::Duration;

/// Result of a cache decision.
///
/// Represents whether an entity should be cached or passed through without
/// caching. Both variants preserve the entity, just wrapped differently.
///
/// # Type Parameters
///
/// * `C` - Type of the cacheable entity (usually the cached representation)
/// * `N` - Type of the non-cacheable entity (usually the original response)
///
/// # Example
///
/// ```
/// use hitbox_core::CachePolicy;
///
/// fn decide_caching(status: u16, body: String) -> CachePolicy<String, String> {
///     if status == 200 {
///         CachePolicy::Cacheable(body)
///     } else {
///         CachePolicy::NonCacheable(body)
///     }
/// }
///
/// match decide_caching(200, "OK".to_string()) {
///     CachePolicy::Cacheable(data) => println!("Cache: {}", data),
///     CachePolicy::NonCacheable(data) => println!("Pass through: {}", data),
/// }
/// ```
#[derive(Debug)]
pub enum CachePolicy<C, N> {
    /// Entity should be cached.
    Cacheable(C),
    /// Entity should not be cached; pass through directly.
    NonCacheable(N),
}

/// Configuration for entity caching TTLs.
///
/// Specifies how long cached entries should live and when they become stale.
/// Used by [`CacheableResponse::cache_policy`](crate::response::CacheableResponse::cache_policy)
/// to set timestamps on cached values.
///
/// # Fields
///
/// * `ttl` - Time until the entry expires (becomes invalid)
/// * `stale_ttl` - Time until the entry becomes stale (should refresh in background)
///
/// # Example
///
/// ```
/// use hitbox_core::EntityPolicyConfig;
/// use std::time::Duration;
///
/// // Expire after 1 hour, become stale after 5 minutes
/// let config = EntityPolicyConfig {
///     ttl: Some(Duration::from_secs(3600)),
///     stale_ttl: Some(Duration::from_secs(300)),
/// };
///
/// // No expiration (cached forever until manually invalidated)
/// let forever = EntityPolicyConfig::default();
/// ```
#[derive(Default)]
pub struct EntityPolicyConfig {
    /// Time until cached entries expire and become invalid.
    pub ttl: Option<Duration>,
    /// Time until cached entries become stale (for background refresh).
    pub stale_ttl: Option<Duration>,
}

// =============================================================================
// Policy Configuration Types
// =============================================================================

/// Concurrency limit for dogpile prevention (1-255).
/// A value of 1 means only one request can fetch from upstream at a time.
pub type ConcurrencyLimit = NonZeroU8;

/// Policy for handling stale cache entries.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum StalePolicy {
    /// Return stale data without any revalidation.
    #[default]
    Return,
    /// Treat stale as expired — block and wait for fresh data (synchronous revalidation).
    Revalidate,
    /// Return stale data immediately and revalidate in background (Stale-While-Revalidate).
    OffloadRevalidate,
}

/// Cache behavior policy configuration.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CacheBehaviorPolicy {
    /// How to handle stale cache entries.
    pub stale: StalePolicy,
}

/// Enabled cache configuration with TTL, stale window, and behavior settings.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EnabledCacheConfig {
    /// Total time-to-live for the cache entry.
    /// After this duration the entry expires and becomes invalid.
    pub ttl: Option<Duration>,
    /// Time after which the cache entry becomes stale.
    /// Between `stale` and `ttl`, the entry can be served as stale data.
    pub stale: Option<Duration>,
    /// Cache behavior policy.
    pub policy: CacheBehaviorPolicy,
    /// Concurrency limit for dogpile prevention.
    pub concurrency: Option<ConcurrencyLimit>,
}

impl Default for EnabledCacheConfig {
    fn default() -> Self {
        Self {
            ttl: Some(Duration::from_secs(5)),
            stale: None,
            policy: CacheBehaviorPolicy::default(),
            concurrency: None,
        }
    }
}

/// Cache policy: enabled with settings or completely disabled.
///
/// When `Disabled`, requests bypass the cache entirely and go directly to upstream.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PolicyConfig {
    /// Caching enabled with the specified configuration.
    Enabled(EnabledCacheConfig),
    /// Caching disabled — all requests go directly to upstream.
    Disabled,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self::Enabled(EnabledCacheConfig::default())
    }
}

impl PolicyConfig {
    /// Create a new builder for an enabled cache configuration.
    pub fn builder() -> PolicyConfigBuilder {
        PolicyConfigBuilder::default()
    }

    /// Create a disabled policy configuration.
    pub fn disabled() -> Self {
        Self::Disabled
    }
}

/// Builder for [`PolicyConfig`].
#[derive(Debug, Clone, Default)]
pub struct PolicyConfigBuilder {
    ttl: Option<Duration>,
    stale: Option<Duration>,
    stale_policy: StalePolicy,
    concurrency: Option<ConcurrencyLimit>,
}

impl PolicyConfigBuilder {
    /// Create a new builder with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the total time-to-live for the cache entry.
    pub fn ttl(self, ttl: Duration) -> Self {
        Self {
            ttl: Some(ttl),
            ..self
        }
    }

    /// Set the time after which the cache entry becomes stale.
    pub fn stale(self, stale: Duration) -> Self {
        Self {
            stale: Some(stale),
            ..self
        }
    }

    /// Set the policy for handling stale cache entries.
    pub fn stale_policy(self, policy: StalePolicy) -> Self {
        Self {
            stale_policy: policy,
            ..self
        }
    }

    /// Set the concurrency limit for dogpile prevention.
    pub fn concurrency(self, limit: ConcurrencyLimit) -> Self {
        Self {
            concurrency: Some(limit),
            ..self
        }
    }

    /// Build the [`PolicyConfig`] with enabled caching.
    pub fn build(self) -> PolicyConfig {
        PolicyConfig::Enabled(EnabledCacheConfig {
            ttl: self.ttl,
            stale: self.stale,
            policy: CacheBehaviorPolicy {
                stale: self.stale_policy,
            },
            concurrency: self.concurrency,
        })
    }
}
