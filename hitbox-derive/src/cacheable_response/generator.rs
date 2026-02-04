//! Generator for CacheableResponse derive macro.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use super::cached_type::CachedType;
use super::trait_impl::CacheableResponseImpl;

/// Generator for CacheableResponse derive macro output.
#[derive(Debug)]
pub struct Generator<'a> {
    cached_type: Option<&'a CachedType<'a>>,
    trait_impl: &'a CacheableResponseImpl<'a>,
}

impl<'a> Generator<'a> {
    pub fn new(
        cached_type: Option<&'a CachedType<'a>>,
        trait_impl: &'a CacheableResponseImpl<'a>,
    ) -> Self {
        Self {
            cached_type,
            trait_impl,
        }
    }
}

impl<'a> ToTokens for Generator<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            cached_type,
            trait_impl,
        } = self;

        if let Some(cached_type) = cached_type {
            tokens.extend(quote! { #cached_type });
        }
        tokens.extend(quote! { #trait_impl });
    }
}
