#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

/// Future types for the cache service.
pub mod future;
/// Tower layer and builder for cache configuration.
pub mod layer;
/// The Tower service implementation that performs caching.
pub mod service;
/// Upstream adapter for bridging Tower services to Hitbox.
pub mod upstream;

pub use ::http::{Method, StatusCode};
pub use hitbox::config::{CacheConfig, CacheConfigRouter, MultiConfig, RouteMatch};
pub use hitbox::{Config, ConfigBuilder};
pub use hitbox_http::DEFAULT_CACHE_STATUS_HEADER;
pub use layer::{Cache, CacheBuilder, NotSet};
pub use upstream::TowerUpstream;
