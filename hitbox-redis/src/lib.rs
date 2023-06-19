#![warn(missing_docs)]
//! hitbox [Backend] implementation for Redis.
//!
//! This crate uses [redis-rs] as base library for asynchronous interaction with redis nodes.
//! It use one [MultiplexedConnection] for better connection utilization.
//!
//! [MultiplexedConnection]: redis::aio::MultiplexedConnection
//! [Backend]: hitbox_backend::Backend
//! [redis-rs]: redis-rs::aio
pub mod backend;
pub mod error;

#[doc(inline)]
pub use crate::backend::{RedisBackend, RedisBackendBuilder};
