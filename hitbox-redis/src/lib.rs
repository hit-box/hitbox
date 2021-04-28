#![warn(missing_docs)]
//! hitbox [Backend] implementation for Redis.
//!
//! This crate uses [redis-rs] as base library for asynchronous interaction with redis nodes.
//! It use one [MultiplexedConnection] for maximum connection utilisation.
//!
/// [MultiplexedConnection]: TODO
/// [Backend]: /hitbox_backend/trait.Backend.html
pub mod actor;
pub mod error;

#[doc(inline)]
pub use crate::actor::{RedisBackend, RedisBackendBuilder};
