//! Generator for CacheableRequest derive macro.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use super::trait_impl::CacheableRequestImpl;

/// Generator for CacheableRequest derive macro output.
#[derive(Debug)]
pub struct Generator<'a> {
    trait_impl: &'a CacheableRequestImpl<'a>,
}

impl<'a> Generator<'a> {
    pub fn new(trait_impl: &'a CacheableRequestImpl<'a>) -> Self {
        Self { trait_impl }
    }
}

impl<'a> ToTokens for Generator<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let trait_impl = self.trait_impl;
        tokens.extend(quote! { #trait_impl });
    }
}
