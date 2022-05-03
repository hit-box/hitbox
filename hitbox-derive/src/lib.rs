#![warn(missing_docs)]
//! This crate provides default implementations for Cacheable and CacheableResponse derive macros.
//!
//! You can see an example of Cacheable derive macro below:
//! ```edition2018,ignore
//! use hitbox::cache::Cacheable;
//! use hitbox::error::CacheError;
//! use serde::Serialize;
//!
//! #[derive(Cacheable, Serialize)]
//! #[hitbox(cache_ttl=120, cache_stale_ttl=100, cache_version=100)]
//! struct Message {
//!     field: i32,
//! };
//! let message = Message { field: 42 };
//! assert_eq!(message.cache_message_key().unwrap(), "Message::v100::field=42".to_string());
//! ```
//!
//! CacheableResponse example:
//! ```edition2018,ignore
//! use hitbox::response::CacheableResponse;
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
mod container;

/// Derive Cacheable macro implementation.
#[proc_macro_derive(Cacheable, attributes(hitbox))]
pub fn cacheable_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    cacheable_macro::impl_macro(&ast)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Derive CacheableResponse macro implementation.
#[proc_macro_derive(CacheableResponse)]
pub fn cacheable_response_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    cacheable_response_macro::impl_macro(&ast)
}
