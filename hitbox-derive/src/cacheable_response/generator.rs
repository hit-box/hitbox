//! Generator for CacheableResponse derive macro.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use super::cached_type::{CachedType, CloneImpl};
use super::trait_impl::CacheableResponseImpl;

/// Generator for CacheableResponse derive macro output.
#[derive(Debug)]
pub struct Generator<'a> {
    cached_type: Option<&'a CachedType<'a>>,
    clone_impl: Option<&'a CloneImpl<'a>>,
    trait_impl: &'a CacheableResponseImpl<'a>,
}

impl<'a> Generator<'a> {
    pub fn new(
        cached_type: Option<&'a CachedType<'a>>,
        clone_impl: Option<&'a CloneImpl<'a>>,
        trait_impl: &'a CacheableResponseImpl<'a>,
    ) -> Self {
        Self {
            cached_type,
            clone_impl,
            trait_impl,
        }
    }
}

impl<'a> ToTokens for Generator<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if let Some(cached_type) = self.cached_type {
            tokens.extend(quote! { #cached_type });
        }
        if let Some(clone_impl) = self.clone_impl {
            tokens.extend(quote! { #clone_impl });
        }
        let trait_impl = self.trait_impl;
        tokens.extend(quote! { #trait_impl });
    }
}
