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
//! use hitbox::prelude::*; // With features=["derive"]
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Cacheable, Serialize)]
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
//! [Cacheable]: cache/trait.Cacheable.html
//! [CacheableResponse]: response/trait.CacheableResponse.html
//! [Backend]: ../hitbox_backend/trait.Backend.html
//! [RedisBackend]: ../hitbox_redis/actor/struct.RedisActor.html
//! [hitbox-actix]: ../hitbox_actix/index.html
//! [dogpile effect]: https://www.sobstel.org/blog/preventing-dogpile-effect/
#![warn(missing_docs)]

pub mod cache;
pub mod dev;
pub mod error;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod response;
pub mod runtime;
pub mod settings;
pub mod states;
pub mod transition_groups;
pub mod value;

pub use cache::Cacheable;
pub use error::CacheError;
pub use response::{CachePolicy, CacheableResponse};
pub use value::{CacheState, CachedValue};

#[cfg(feature = "derive")]
#[doc(hidden)]
pub use serde_qs as hitbox_serializer;

/// The `hitbox` prelude.
pub mod prelude {
    #[cfg(feature = "derive")]
    pub use crate::hitbox_serializer;
    pub use crate::{CacheError, Cacheable, CacheableResponse};
}
