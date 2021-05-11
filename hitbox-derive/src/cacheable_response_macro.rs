use proc_macro::TokenStream;

use quote::quote;

/// Implementing CacheableResponse trait.
pub fn impl_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let gen = quote! {
        impl CacheableResponse for #name {
            type Cached = #name;
            fn cache_policy(&self) -> CachePolicy<&Self::Cached, ()> {
                CachePolicy::Cacheable(self)
            }
            fn into_cache_policy(self) -> CachePolicy<Self::Cached, Self> {
                CachePolicy::Cacheable(self)
            }
            fn from_cached(cached: Self::Cached) -> Self {
                cached
            }
        }
    };
    gen.into()
}
