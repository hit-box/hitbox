#![warn(missing_docs)]
//! # Hitbox-Actix
//!
//! [![Build status](https://github.com/hit-box/hitbox/actions/workflows/CI.yml/badge.svg)](https://github.com/hit-box/hitbox/actions?query=workflow)
//! [![Coverage Status](https://codecov.io/gh/hit-box/hitbox/branch/master/graph/badge.svg?token=tgAm8OBLkY)](https://codecov.io/gh/hit-box/hitbox)
//! 
//! Hitbox-Actix is an asynchronous caching framework for [Actix] actor framework.
//! It's designed for distributed and for single-machine applications.
//! 
//! ## Features
//! - [x] Automatic cache key generation.
//! - [x] Multiple cache backend implementations.
//! - [x] Stale cache mechanics.
//! - [ ] Cache locks for [dogpile effect] preventions.
//! - [ ] Distributed cache locks.
//! - [ ] Detailed metrics out of the box.
//!
//! ## Backend implementations:
//! - [x] [Redis](https://github.com/hit-box/hitbox/tree/master/hitbox-backend)
//! - [ ] In-memory backend
//!
//! ## Feature flags
//! * derive - Support for [Cacheable] trait derive macros.
//! * redis - Support for default redis backend.
//!
//! ## Restrictions
//! Default cache key implementation based on serde_qs crate
//! and have some [restrictions](https://docs.rs/serde_qs/latest/serde_qs/#supported-types).
//!
//! ## Documentation
//! * [API Documentation](https://docs.rs/hitbox_acitx/)
//! * [Examples](https://github.com/hit-box/hitbox/tree/master/examples/examples)
//!
//! ### Flow diagrams:
//! [![Simple flow](http://www.plantuml.com/plantuml/proxy?src=https://raw.githubusercontent.com/hit-box/hitbox/master/documentation/simple_flow.puml)](http://www.plantuml.com/plantuml/proxy?src=https://raw.githubusercontent.com/hit-box/hitbox/master/documentation/simple_flow.puml)
//!
//! ## Example
//!
//! ### Dependencies:
//!
//! ```toml
//! [dependencies]
//! hitbox_actix = "0.1"
//! ```
//!
//! ### Code:
//!
//! First, you should derive [Cacheable] trait for your actix [Message]:
//!
//! ```rust
//! use actix::prelude::*;
//! use actix_derive::{Message, MessageResponse};
//! use hitbox_actix::prelude::*;
//! use serde::{Deserialize, Serialize};
//! 
//! #[derive(Message, Cacheable, Serialize)]
//! #[rtype(result = "Result<Pong, Error>")]
//! struct Ping {
//!     id: i32,
//! }
//! 
//! #[derive(MessageResponse, Deserialize, Serialize, Debug)]
//! struct Pong(i32);
//! 
//! #[derive(Debug)]
//! struct Error;
//! ```
//! 
//! Next step is declare Upstream actor and implement actix Handler for Ping:
//! 
//! ```rust
//! #[derive(Debug)]
//! struct UpstreamActor;
//! 
//! impl Actor for UpstreamActor {
//!     type Context = Context<Self>;
//! }
//! 
//! impl Handler<Ping> for UpstreamActor {
//!     type Result = ResponseFuture<<Ping as Message>::Result>;
//! 
//!     fn handle(&mut self, msg: Ping, _ctx: &mut Self::Context) -> Self::Result {
//!         println!("Handler::Ping");
//!         Box::pin(async move {
//!             actix_rt::time::sleep(core::time::Duration::from_secs(3)).await;
//!             Ok(Pong(msg.id))
//!         })
//!     }
//! }
//! ```
//! The last step is initialize and start CacheActor and UpstreamActor:
//! 
//! ```rust
//! use tracing_subscriber::EnvFilter;
//! 
//! #[actix_rt::main]
//! async fn main() -> Result<(), CacheError> {
//!     let filter = EnvFilter::new("hitbox=trace");
//!     tracing_subscriber::fmt()
//!         .with_max_level(tracing::Level::TRACE)
//!         .with_env_filter(filter)
//!         .init();
//! 
//!     let backend = RedisBackend::new()
//!     	.await?
//!     	.start();
//! 
//!     let cache = Cache::builder()
//!         .with_stale()
//!         .finish(backend)
//!         .start();
//!     let upstream = UpstreamActor.start();
//! 
//!     /// And send `Ping` message into cache actor
//!     let msg = Ping { id: 42 };
//!     let res = cache.send(msg.into_cache(&upstream)).await??;
//!     println!("{:#?}", res);
//!     Ok(())
//! }
//! ```
//! 
//! [Cacheable]: hitbox::Cacheable
//! [CacheableResponse]: hitbox::CacheableResponse
//! [Backend]: hitbox_backend::Backend
//! [RedisBackend]: hitbox_redis::RedisActor
//! [dogpile effect]: https://www.sobstel.org/blog/preventing-dogpile-effect/
//! [Message]: actix::Message
//! [Actix]: https://github.com/actix/actix/
  
pub mod actor;
pub mod builder;
pub mod handlers;
pub mod messages;
pub mod runtime;

pub use actor::CacheActor;
pub use builder::CacheBuilder;
pub use hitbox::{CacheError, Cacheable};
pub use messages::{IntoCache, QueryCache};
pub use runtime::ActixAdapter;

#[cfg(feature = "redis")]
pub use hitbox_redis::RedisBackend;

/// Default type alias with RedisBackend.
/// You can disable it or define it manually in your code.
#[cfg(feature = "redis")]
pub type Cache = CacheActor<RedisBackend>;

/// Prelude for hitbox_actix.
pub mod prelude {
    #[cfg(feature = "redis")]
    pub use crate::{Cache, RedisBackend};
    pub use crate::{CacheActor, CacheBuilder, CacheError, Cacheable, IntoCache, QueryCache};
    pub use hitbox::hitbox_serializer;
}
