use proc_macro::TokenStream;
use quote::quote;
use syn;
use crate::macro_attributes::find_attribute;

/// Implementing Cacheable trait.
///
/// Uses `serde_qs` crate to create a unique cache key.
pub fn impl_cacheable_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let cache_ttl: u32 = find_attribute(&ast, "cache_ttl")
        .unwrap_or_else(|| 60);

    let cache_stale_ttl: u32 = find_attribute(&ast, "cache_stale_ttl")
        .unwrap_or_else(|| 5);

    let cache_version: u16 = find_attribute(&ast, "cache_version")
        .unwrap_or_else(|| 0) as u16;

    let gen = quote! {
        impl Cacheable for #name {
            /// Method should return unique identifier for struct object.
            fn cache_key(&self) -> String {
                serde_qs::to_string(self).unwrap()
            }

            /// Describe time-to-live (ttl) value for cache storage in seconds.
            fn cache_ttl(&self) -> u32 {
                #cache_ttl
            }

            /// Describe expire\stale timeout value for cache storage in seconds.
            fn cache_stale_ttl(&self) -> u32 {
                self.cache_ttl() - #cache_stale_ttl
            }

            /// Describe current cache version for this message type.
            fn cache_version(&self) -> u16 {
                #cache_version
            }
        }
    };
    gen.into()
}
