use crate::macro_attributes::find_attribute;
use proc_macro::TokenStream;
use quote::quote;
use syn;

/// Implementing Cacheable trait.
///
/// Uses `serde_qs` crate to create a unique cache key.
/// Default implementation of methods `cache_ttl`, `cache_stale_ttl` and `cache_version`
/// are used if macros of the same name are not used.
pub fn impl_cacheable_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let cache_ttl_implement = match find_attribute(&ast, "cache_ttl") {
        Some(cache_ttl) => quote! {
            fn cache_ttl(&self) -> u32 {
                #cache_ttl
            }
        },
        None => proc_macro2::TokenStream::new(),
    };

    let cache_stale_ttl_implement = match find_attribute(&ast, "cache_stale_ttl") {
        Some(cache_stale_ttl) => quote! {
            fn cache_stale_ttl(&self) -> u32 {
                #cache_stale_ttl
            }
        },
        None => proc_macro2::TokenStream::new(),
    };

    let cache_version_implement = match find_attribute(&ast, "cache_version") {
        Some(cache_version) => quote! {
            fn cache_version(&self) -> u32 {
                #cache_version
            }
        },
        None => proc_macro2::TokenStream::new(),
    };

    let gen = quote! {
        impl Cacheable for #name {
            fn cache_key(&self) -> Result<String, CacheError> {
                actix_cache::serde_qs::to_string(self)
                    .map_err(|error| CacheError::CacheKeyGenerationError(error.to_string()))
            }
            #cache_ttl_implement
            #cache_stale_ttl_implement
            #cache_version_implement
        }
    };
    gen.into()
}
