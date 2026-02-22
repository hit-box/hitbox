//! Generated cached type for types with skipped fields.

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::Ident;

use super::parser::Source;

/// Generated cached type for types with skipped fields.
///
/// All fields are included. Skipped fields get `#[serde(skip, default)]`
/// so they are not serialized. A custom `Clone` impl ([`CloneImpl`])
/// defaults skipped fields instead of cloning them.
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
                    quote! {
                        #[serde(skip, default)]
                        #vis #ident: #ty
                    }
                } else {
                    quote! { #vis #ident: #ty }
                }
            })
            .collect();

        let expanded = quote! {
            /// Auto-generated cached representation.
            #[derive(hitbox::serde::Serialize, hitbox::serde::Deserialize)]
            #[serde(crate = "hitbox::serde")]
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

/// Custom `Clone` impl for the generated cached type.
///
/// Non-skipped fields are cloned normally. Skipped fields are reconstructed
/// via `SkippedFieldDefault::skipped_default()`, preventing sensitive data
/// from leaking into cache storage when the FSM clones for persistence.
#[derive(Debug)]
pub struct CloneImpl<'a> {
    source: &'a Source,
    cached_type: &'a CachedType<'a>,
}

impl<'a> CloneImpl<'a> {
    pub fn new(source: &'a Source, cached_type: &'a CachedType<'a>) -> Self {
        Self {
            source,
            cached_type,
        }
    }
}

impl<'a> ToTokens for CloneImpl<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let cached_name = self.cached_type.ident();
        let (impl_generics, ty_generics, where_clause) = self.source.generics.split_for_impl();

        let clone_fields: Vec<_> = self
            .source
            .fields()
            .map(|f| {
                let ident = f.ident.as_ref().expect("named field");
                if f.skip {
                    quote! { #ident: hitbox_fn::SkippedFieldDefault::skipped_default() }
                } else {
                    quote! { #ident: hitbox_fn::CachedFieldClone::cached_clone(&self.#ident) }
                }
            })
            .collect();

        let expanded = quote! {
            impl #impl_generics Clone for #cached_name #ty_generics #where_clause {
                fn clone(&self) -> Self {
                    Self {
                        #(#clone_fields,)*
                    }
                }
            }
        };

        tokens.extend(expanded);
    }
}
