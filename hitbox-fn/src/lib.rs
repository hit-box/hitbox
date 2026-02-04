//! Function memoization for hitbox caching framework.
//!
//! This crate provides tools for caching async function results using the hitbox FSM.
//!
//! # Overview
//!
//! - [`Args`] - Wrapper for function arguments (tuple)
//! - [`KeyExtract`] - Trait for types to describe their cache key contribution
//! - [`FnExtractor`] - Bridges `KeyExtract` to hitbox's `Extractor` trait
//! - [`FnUpstream`] - Adapts async functions to hitbox's `Upstream` trait
//! - [`Cache`] - Pre-configured cache with backend and policy
//!
//! # Usage
//!
//! ## With derive macros (requires `derive` feature)
//!
//! ```ignore
//! use hitbox_fn::prelude::*;
//!
//! #[derive(KeyExtract)]
//! struct UserId(u64);
//!
//! #[cached]
//! async fn fetch_user(id: UserId) -> Result<User, Error> {
//!     // expensive operation
//! }
//!
//! // Usage
//! let user = fetch_user(UserId(42))
//!     .cache(&cache)
//!     .await?;
//! ```
//!
//! ## Manual implementation
//!
//! ```ignore
//! use hitbox_fn::{Args, KeyExtract, FnExtractor, Cache};
//!
//! struct UserId(u64);
//!
//! impl KeyExtract for UserId {
//!     fn extract(&self) -> Vec<KeyPart> {
//!         vec![KeyPart::new("user_id", Some(self.0.to_string()))]
//!     }
//! }
//! ```

#![warn(missing_docs)]

mod args;
mod cache;
mod extractor;
mod upstream;

pub use args::Args;
pub use cache::{
    Cache, CacheBuilder, NoBackend, NoContext, NoPolicy, WithBackend, WithContext, WithPolicy,
};
pub use extractor::{FnExtractor, KeyExtract};
pub use upstream::FnUpstream;

// Re-export derive macros when feature is enabled
#[cfg(feature = "derive")]
pub use hitbox_derive::{
    CacheableRequest, CacheableResponse, KeyExtract as KeyExtractDerive, cached,
};

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::{Args, Cache, FnExtractor, FnUpstream, KeyExtract};

    #[cfg(feature = "derive")]
    pub use hitbox_derive::{
        CacheableRequest, CacheableResponse, KeyExtract as KeyExtractDerive, cached,
    };

    // Re-export commonly used hitbox types
    pub use hitbox::policy::PolicyConfig;
    pub use hitbox::{CacheContext, CacheStatus, ResponseSource};
    pub use hitbox_core::KeyPart;
}
