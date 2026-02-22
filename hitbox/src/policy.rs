//! Policy configuration for cache behavior.
//!
//! Re-exports core policy types from [`hitbox_core::policy`].

pub use hitbox_core::policy::{
    CacheBehaviorPolicy, ConcurrencyLimit, EnabledCacheConfig, PolicyConfig, PolicyConfigBuilder,
    StalePolicy,
};
