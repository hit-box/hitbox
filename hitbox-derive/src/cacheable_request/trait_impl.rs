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
            impl #impl_generics hitbox::CacheableRequest for #name #ty_generics #where_clause {
                type CachePolicyFuture<'__a, __P, __E, __TE> = ::std::pin::Pin<
                    ::std::boxed::Box<
                        dyn ::std::future::Future<
                            Output = (
                                hitbox::RequestCachePolicy<Self>,
                                ::std::vec::Vec<hitbox_core::tag::CacheTag>,
                            )
                        > + Send + '__a
                    >
                >
                where
                    Self: '__a,
                    __P: hitbox::predicate::Predicate<Subject = Self> + Send + Sync + '__a,
                    __E: hitbox::Extractor<Subject = Self> + Send + Sync + '__a,
                    __TE: hitbox_core::tag::TagExtractor<Subject = Self> + Send + Sync + '__a;

                fn cache_policy<'__a, __P, __E, __TE>(
                    self,
                    predicates: __P,
                    extractors: __E,
                    tag_extractor: __TE,
                ) -> Self::CachePolicyFuture<'__a, __P, __E, __TE>
                where
                    Self: '__a,
                    __P: hitbox::predicate::Predicate<Subject = Self> + Send + Sync + '__a,
                    __E: hitbox::Extractor<Subject = Self> + Send + Sync + '__a,
                    __TE: hitbox_core::tag::TagExtractor<Subject = Self> + Send + Sync + '__a,
                {
                    ::std::boxed::Box::pin(async move {
                        match predicates.check(self).await {
                            hitbox::predicate::PredicateResult::Cacheable(subject) => {
                                let (subject, key) = extractors.get(subject).await.into_cache_key();
                                let (subject, tags) = tag_extractor.extract_tags(subject).await;
                                (
                                    hitbox::CachePolicy::Cacheable(
                                        hitbox::CacheablePolicyData::new(key, subject)
                                    ),
                                    tags,
                                )
                            }
                            hitbox::predicate::PredicateResult::NonCacheable(subject) => {
                                (hitbox::CachePolicy::NonCacheable(subject), ::std::vec::Vec::new())
                            }
                        }
                    })
                }
            }
        };

        tokens.extend(expanded);
    }
}
