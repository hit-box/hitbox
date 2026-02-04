//! CacheableRequest trait implementation generator.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use super::parser::Source;

/// Generator for CacheableRequest trait implementation.
#[derive(Debug)]
pub struct CacheableRequestImpl<'a> {
    source: &'a Source,
}

impl<'a> CacheableRequestImpl<'a> {
    pub fn new(source: &'a Source) -> Self {
        Self { source }
    }
}

impl<'a> ToTokens for CacheableRequestImpl<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.source.ident;
        let (impl_generics, ty_generics, where_clause) = self.source.generics.split_for_impl();

        let expanded = quote! {
            impl #impl_generics hitbox_core::CacheableRequest for #name #ty_generics #where_clause {
                async fn cache_policy<__P, __E>(
                    self,
                    predicates: __P,
                    extractors: __E,
                ) -> hitbox_core::RequestCachePolicy<Self>
                where
                    __P: hitbox_core::predicate::Predicate<Subject = Self> + Send + Sync,
                    __E: hitbox_core::Extractor<Subject = Self> + Send + Sync,
                {
                    match predicates.check(self).await {
                        hitbox_core::predicate::PredicateResult::Cacheable(subject) => {
                            let (subject, key) = extractors.get(subject).await.into_cache_key();
                            hitbox_core::CachePolicy::Cacheable(
                                hitbox_core::CacheablePolicyData::new(key, subject)
                            )
                        }
                        hitbox_core::predicate::PredicateResult::NonCacheable(subject) => {
                            hitbox_core::CachePolicy::NonCacheable(subject)
                        }
                    }
                }
            }
        };

        tokens.extend(expanded);
    }
}
