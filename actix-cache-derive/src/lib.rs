//! This crate provides Cacheable derive macros.
//!
//! ```edition2018
//! # use actix_cache::Cacheable;
//! # use serde_derive::{Serialize, Deserialize};
//! #
//! # #[derive(Cacheable, Serialize)]
//! # #[cache_ttl(120)]
//! # #[cache_stale_ttl(100)]
//! # #[cache_version(100)]
//! # struct Message {
//! #     field: i32,
//! # };
//! # let message = Message { field: 42 };
//! # assert_eq!(message.cache_key(), "id=42".to_string());
//! ```
mod cacheable_macro;
mod macro_attributes;
use syn::export::TokenStream;

#[proc_macro_derive(Cacheable, attributes(cache_ttl, cache_stale_ttl, cache_version))]
pub fn cacheable_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    cacheable_macro::impl_cacheable_macro(&ast)
}
