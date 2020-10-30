use proc_macro::TokenStream;

use quote::quote;

use crate::macro_attributes::find_attribute;

/// Implementing Cacheable trait.
///
/// Uses `serde_qs` crate to create a unique cache key.
/// Default implementation of methods `cache_ttl`, `cache_stale_ttl` and `cache_version`
/// are used if macros of the same name are not used.
pub fn impl_cacheable_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let message_type = format!("{}", name);

    let cache_key_implement = quote! {
        fn cache_key(&self) -> Result<String, actix_cache::CacheError> {
            actix_cache::serde_qs::to_string(self)
                .map(|key| format!("{}::v{}::{}", self.cache_key_prefix(), self.cache_version(), key))
                .map_err(|error| actix_cache::CacheError::CacheKeyGenerationError(error.to_string()))
        }
    };

    let cache_key_prefix_implement = quote! {
        fn cache_key_prefix(&self) -> String {
            #message_type.to_owned()
        }
    };

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
            #cache_key_implement
            #cache_key_prefix_implement
            #cache_ttl_implement
            #cache_stale_ttl_implement
            #cache_version_implement
        }
    };
    gen.into()
}
