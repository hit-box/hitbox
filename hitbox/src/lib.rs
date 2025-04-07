//! An a implementation and infrastructure for asynchronous and clear cache integration.
//!
//! # A quick tour of hitbox
//!
//! Our crates consist of next main part:
//! * [Cacheable] trait.
//! * [Backend] trait and its implementation ([RedisBackend]).
//! * [CacheableResponse] trait.
//! * Cache implementation. ([hitbox-actix])
//!
//! ## Features
//! - [x] Automatic cache key generation.
//! - [x] Framework integrations:
//!     - [x] Actix ([hitbox-actix])
//!     - [ ] Actix-Web
//! - [x] Multiple cache backend implementations:
//!     - [x] [RedisBackend]
//!     - [ ] In-memory backend
//! - [x] Stale cache mechanics.
//! - [ ] Cache locks for [dogpile effect] preventions.
//! - [ ] Distributed cache locks.
//! - [ ] Detailed metrics out of the box.
//!
//! ## Feature flags
//! * derive - Support for [Cacheable] trait derive macros.
//! * metrics - Support for metrics.
//!
//! ## Restrictions
//! Default cache key implementation based on serde_qs crate
//! and have some [restrictions](https://docs.rs/serde_qs/latest/serde_qs/#supported-types).
//!
//! ## Example
//! First of all, you should derive [Cacheable] trait for your struct or enum:
//!
//! ```rust
//! use hitbox::prelude::*;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Cacheable, Serialize)] // With features=["derive"]
//! struct Ping {
//!     id: i32,
//! }
//! ```
//! Or implement that trait manually:
//!
//! ```rust
//! # use hitbox::{Cacheable, CacheError};
//! # struct Ping { id: i32 }
//! impl Cacheable for Ping {
//!     fn cache_key(&self) -> Result<String, CacheError> {
//!         Ok(format!("{}::{}", self.cache_key_prefix(), self.id))
//!     }
//!
//!     fn cache_key_prefix(&self) -> String { "Ping".to_owned() }
//! }
//! ```
//!
//! [Cacheable]: crate::Cacheable
//! [CacheableResponse]: crate::CacheableResponse
//! [Backend]: hitbox_backend::Backend
//! [RedisBackend]: https://docs.rs/hitbox_redis/
//! [hitbox-actix]: https://docs.rs/hitbox_actix/
//! [dogpile effect]: https://www.sobstel.org/blog/preventing-dogpile-effect/
#![allow(missing_docs)] // TODO: replace to warn
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod backend;
pub mod error;
pub mod fsm;
#[cfg(feature = "metrics")]
#[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
pub mod metrics;
pub use error::CacheError;
pub use hitbox_core::{
    CacheKey, CachePolicy, CacheState, CacheValue, CacheablePolicyData, CacheableRequest,
    CacheableResponse, EntityPolicyConfig, Extractor, KeyPart, KeyParts, Predicate,
    RequestCachePolicy, ResponseCachePolicy,
};
pub mod policy;

pub mod predicate {
    pub use hitbox_core::{Predicate, PredicateResult};
}

pub mod extractor {
    pub use hitbox_core::Extractor;
}

/// The `hitbox` prelude.
pub mod prelude {
    pub use crate::{CacheError, CacheableRequest, CacheableResponse};
}
