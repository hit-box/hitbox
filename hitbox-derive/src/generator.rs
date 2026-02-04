use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Index;

use crate::parser::{Field, Source};

#[derive(Debug)]
pub(crate) struct Generator<'a> {
    pub(crate) source: &'a Source,
    pub(crate) fields: &'a [Field<'a>],
}

impl<'a> Generator<'a> {
    pub(crate) fn new(source: &'a Source, fields: &'a [Field<'a>]) -> Self {
        Generator { source, fields }
    }
}

impl<'a> ToTokens for Generator<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { source, fields } = self;
        let name = &source.ident;

        let field_extracts: Vec<_> = fields
            .iter()
            .filter(|f| !f.is_skipped())
            .map(|field| {
                let key_name = field.key_name();
                let field_access = if let Some(ident) = &field.attributes.ident {
                    quote! { &self.#ident }
                } else {
                    let idx = Index::from(field.index);
                    quote! { &self.#idx }
                };

                quote! {
                    parts.push(hitbox_core::KeyPart::new(#key_name, Some(#field_access.to_string())));
                }
            })
            .collect();

        let expanded = if field_extracts.is_empty() {
            quote! {
                impl hitbox_fn::KeyExtract for #name {
                    fn extract(&self) -> Vec<hitbox_core::KeyPart> {
                        vec![hitbox_core::KeyPart::new(stringify!(#name), None::<&str>)]
                    }
                }
            }
        } else {
            quote! {
                impl hitbox_fn::KeyExtract for #name {
                    fn extract(&self) -> Vec<hitbox_core::KeyPart> {
                        let mut parts = Vec::new();
                        #(#field_extracts)*
                        parts
                    }
                }
            }
        };

        tokens.extend(expanded);
    }
}
