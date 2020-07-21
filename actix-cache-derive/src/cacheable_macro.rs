use proc_macro::TokenStream;
use quote::quote;
use syn;
use crate::macro_attributes::find_attribute;

/// Implementing Cacheable trait.
///
/// Uses `serde_qs` crate to create a unique cache key.
pub fn impl_cacheable_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let cache_ttl_implement = match find_attribute(&ast, "cache_ttl") {
        Some(cache_ttl) => quote! {
            fn cache_ttl(&self) -> u32 {
                #cache_ttl
            }
        },
        None => proc_macro2::TokenStream::new()
    };

    let cache_ttl_stale_implement = match find_attribute(&ast, "cache_stale_ttl") {
        Some(cache_stale_ttl) => quote! {
            fn cache_stale_ttl(&self) -> u32 {
                #cache_stale_ttl
            }
        },
        None => proc_macro2::TokenStream::new()
    };

    let cache_version_implement = match find_attribute(&ast, "cache_version") {
        Some(cache_version) => quote! {
            let cache_version = u16::from(cache_version);
            fn cache_version(&self) -> u16 {
                #cache_version
            }
        },
        None => proc_macro2::TokenStream::new()
    };


    let gen = quote! {
        impl Cacheable for #name {
            /// Method should return unique identifier for struct object.
            fn cache_key(&self) -> String {
                serde_qs::to_string(self).unwrap()
            }

            /// Describe time-to-live (ttl) value for cache storage in seconds.
            #cache_ttl_implement

            /// Describe expire\stale timeout value for cache storage in seconds.
            #cache_ttl_stale_implement

            /// Describe current cache version for this message type.
            #cache_version_implement
        }
    };
    gen.into()
}
