#![doc = include_str!("../README.md")]

mod middleware;
mod upstream;

pub use middleware::{CacheMiddleware, CacheMiddlewareBuilder, NotSet};
pub use upstream::ReqwestUpstream;

// Re-export hitbox-http types for convenience
pub use hitbox_http::{
    BufferedBody, CacheableHttpRequest, CacheableHttpResponse, DEFAULT_CACHE_STATUS_HEADER,
    SerializableHttpResponse, extractors, predicates,
};

/// Re-export reqwest body type for convenience in type annotations
pub use reqwest::Body as ReqwestBody;

// Re-export common types
pub use hitbox::config::{CacheConfig, CacheConfigs};
pub use hitbox::policy::PolicyConfig;
pub use hitbox::{Config, ConfigBuilder, SelectiveConfig};
pub use hitbox_core::DisabledOffload;

// Re-export concurrency types
pub use hitbox::concurrency::{
    BroadcastConcurrencyManager, ConcurrencyManager, NoopConcurrencyManager,
};
