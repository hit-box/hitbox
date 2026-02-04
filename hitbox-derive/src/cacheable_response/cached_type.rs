//! Generated cached type for types with skipped fields.

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::Ident;

use super::parser::Source;

/// Generated cached type with skipped fields excluded from serialization.
///
/// Skipped fields are present in the struct but marked with `#[serde(skip)]`
/// so they are not serialized to cache. This preserves their values on cache
/// miss while excluding them from storage.
#[derive(Debug)]
pub struct CachedType<'a> {
    source: &'a Source,
}

impl<'a> CachedType<'a> {
    pub fn new(source: &'a Source) -> Self {
        Self { source }
    }

    pub fn ident(&self) -> Ident {
        format_ident!("{}Cached", self.source.ident)
    }
}

impl<'a> ToTokens for CachedType<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let cached_name = self.ident();
        let (impl_generics, _, where_clause) = self.source.generics.split_for_impl();

        let fields: Vec<_> = self
            .source
            .fields()
            .map(|f| {
                let ident = f.ident.as_ref().expect("named field");
                let ty = &f.ty;
                let vis = &f.vis;

                if f.skip {
                    // Skipped fields: present in struct but excluded from serialization
                    quote! {
                        #[serde(skip, default)]
                        #[cfg_attr(feature = "rkyv_format", rkyv(with = rkyv::with::Skip))]
                        #vis #ident: #ty
                    }
                } else {
                    quote! { #vis #ident: #ty }
                }
            })
            .collect();

        let expanded = quote! {
            /// Auto-generated cached representation.
            #[derive(Clone, serde::Serialize, serde::Deserialize)]
            #[cfg_attr(
                feature = "rkyv_format",
                derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
            )]
            pub struct #cached_name #impl_generics #where_clause {
                #(#fields,)*
            }
        };

        tokens.extend(expanded);
    }
}
