use std::fmt::Debug;

use chrono::{DateTime, Utc};

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(Debug, Clone)]
pub struct CachedValue<T> {
    pub data: T,
    pub expired: DateTime<Utc>,
}

impl<T> CachedValue<T> {
    pub fn new(data: T, expired: DateTime<Utc>) -> Self {
        CachedValue { data, expired }
    }

    pub fn into_inner(self) -> T {
        self.data
    }
}

/// TTL eviction settings.
///
/// More information you cat see in [`crate::Cacheable`] trait implementation.
pub struct TtlSettings {
    /// Describe current cached data TTL value.
    ///
    /// More information you can see in [`crate::Cacheable::cache_ttl`].
    pub ttl: u32,

    /// Describe current cached data stale TTL value.
    ///
    /// More information you can see in [`crate::Cacheable::cache_stale_ttl`].
    pub stale_ttl: u32,
}

/// Cached data eviction policy settings.
pub enum EvictionPolicy {
    /// Eviction by TTL settings.
    Ttl(TtlSettings),
}

impl<T> From<(T, EvictionPolicy)> for CachedValue<T> {
    fn from(model: (T, EvictionPolicy)) -> Self {
        let (data, eviction_policy) = model;
        match eviction_policy {
            EvictionPolicy::Ttl(settings) => {
                let duration = chrono::Duration::seconds(settings.stale_ttl as i64);
                let expired = chrono::Utc::now() + duration;
                Self { data, expired }
            }
        }
    }
}
