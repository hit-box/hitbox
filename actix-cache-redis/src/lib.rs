#![warn(missing_docs)]
//! Actix-cache [Backend] implementation for Redis.
//!
//! This crate uses [redis-rs] as base library for asynchronous interaction with redis nodes.
//! It use one [MultiplexedConnection] for maximum connection utilisation.
//!
/// [MultiplexedConnection]: TODO
/// [Backend]: /actix_cache_backend/trait.Backend.html
pub mod actor;
pub mod error;

#[doc(inline)]
pub use crate::actor::{RedisBackend, RedisBackendBuilder};
