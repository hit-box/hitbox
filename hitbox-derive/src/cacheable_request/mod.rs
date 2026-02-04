//! CacheableRequest derive macro implementation.

mod generator;
mod parser;
mod trait_impl;

use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error};

use generator::Generator;
use parser::Source;
use trait_impl::CacheableRequestImpl;

pub fn expand(input: &DeriveInput) -> Result<TokenStream, Error> {
    let source = Source::from_derive_input(input)?;
    let trait_impl = CacheableRequestImpl::new(&source);
    let generator = Generator::new(&trait_impl);
    Ok(quote! { #generator })
}
