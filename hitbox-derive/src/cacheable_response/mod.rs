//! CacheableResponse derive macro implementation.

mod cached_type;
mod generator;
mod parser;
mod trait_impl;

use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error};

use cached_type::CachedType;
use generator::Generator;
use parser::Source;
use trait_impl::CacheableResponseImpl;

pub fn expand(input: &DeriveInput) -> Result<TokenStream, Error> {
    let source = Source::from_derive_input(input)?;

    if source.has_skipped_fields() {
        // Generate separate Cached type and impl with field mapping
        let cached_type = CachedType::new(&source);
        let trait_impl = CacheableResponseImpl::with_cached_type(&source, &cached_type);
        let generator = Generator::new(Some(&cached_type), &trait_impl);
        Ok(quote! { #generator })
    } else {
        // Generate simple impl where Cached = Self
        let trait_impl = CacheableResponseImpl::simple(&source);
        let generator = Generator::new(None, &trait_impl);
        Ok(quote! { #generator })
    }
}
