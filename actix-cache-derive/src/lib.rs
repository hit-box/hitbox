#![warn(missing_docs)]
//! This crate provides default implementations for Cacheable and CacheableResponse derive macros.
//!
//! You can see an example of Cacheable derive macro below:
//! ```edition2018,ignore
//! use actix_cache::cache::Cacheable;
//! use actix_cache::error::CacheError;
//! use serde::Serialize;
//!
//! #[derive(Cacheable, Serialize)]
//! #[cache_ttl(120)]
//! #[cache_stale_ttl(100)]
//! #[cache_version(100)]
//! struct Message {
//!     field: i32,
//! };
//! let message = Message { field: 42 };
//! assert_eq!(message.cache_message_key().unwrap(), "Message::v100::field=42".to_string());
//! ```
//!
//! CacheableResponse example:
//! ```edition2018,ignore
//! use actix_cache::response::CacheableResponse;
//! use serde::Serialize;
//!
//! #[derive(CacheableResponse, Serialize)]
//! pub enum MyResult {
//!     OptionOne(i32),
//!     OptionTwo(String),
//! }
//! ```
use proc_macro::TokenStream;

mod cacheable_macro;
mod cacheable_response_macro;
mod macro_attributes;

/// Derive Cacheable macro implementation.
#[proc_macro_derive(Cacheable, attributes(cache_ttl, cache_stale_ttl, cache_version))]
pub fn cacheable_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    cacheable_macro::impl_macro(&ast)
}

/// Derive CacheableResponse macro implementation.
#[proc_macro_derive(CacheableResponse)]
pub fn cacheable_response_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    cacheable_response_macro::impl_macro(&ast)
}

