//! CacheableResponse trait implementation generator.

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use super::cached_type::CachedType;
use super::parser::Source;

/// Generator for CacheableResponse trait implementation.
#[derive(Debug)]
pub struct CacheableResponseImpl<'a> {
    source: &'a Source,
    cached_type: Option<&'a CachedType<'a>>,
}

impl<'a> CacheableResponseImpl<'a> {
    /// Create impl where Cached = Self (no skipped fields).
    pub fn simple(source: &'a Source) -> Self {
        Self {
            source,
            cached_type: None,
        }
    }

    /// Create impl with separate Cached type (has skipped fields).
    pub fn with_cached_type(source: &'a Source, cached_type: &'a CachedType<'a>) -> Self {
        Self {
            source,
            cached_type: Some(cached_type),
        }
    }

    fn generate_simple(&self, tokens: &mut TokenStream) {
        let name = &self.source.ident;
        let (impl_generics, ty_generics, where_clause) = self.source.generics.split_for_impl();

        let expanded = quote! {
            impl #impl_generics hitbox::CacheableResponse for #name #ty_generics #where_clause {
                type Cached = Self;
                type Subject = Self;
                type IntoCachedFuture = std::future::Ready<hitbox::CachePolicy<Self, Self>>;
                type FromCachedFuture = std::future::Ready<Self>;

                async fn cache_policy<__P, __TE>(
                    self,
                    predicates: __P,
                    tag_extractor: ::std::option::Option<__TE>,
                    config: &hitbox::EntityPolicyConfig,
                ) -> (
                    hitbox::ResponseCachePolicy<Self>,
                    ::std::vec::Vec<hitbox::tag::CacheTag>,
                )
                where
                    __P: hitbox::predicate::Predicate<Subject = Self::Subject> + Send + Sync,
                    __TE: hitbox::tag::TagExtractor<Subject = Self::Subject> + Send + Sync,
                {
                    match predicates.check(self).await {
                        hitbox::predicate::PredicateResult::Cacheable(data) => {
                            let (data, tags) = match tag_extractor {
                                ::std::option::Option::Some(__te) => {
                                    __te.extract_tags(data).await
                                }
                                ::std::option::Option::None => {
                                    (data, ::std::vec::Vec::new())
                                }
                            };
                            let cached = data.clone();
                            (
                                hitbox::CachePolicy::Cacheable(
                                    hitbox::CacheValue::from_config(cached, config),
                                ),
                                tags,
                            )
                        }
                        hitbox::predicate::PredicateResult::NonCacheable(data) => {
                            (hitbox::CachePolicy::NonCacheable(data), ::std::vec::Vec::new())
                        }
                    }
                }

                fn into_cached(self) -> Self::IntoCachedFuture {
                    std::future::ready(hitbox::CachePolicy::Cacheable(self))
                }

                fn from_cached(cached: Self) -> Self::FromCachedFuture {
                    std::future::ready(cached)
                }
            }
        };

        tokens.extend(expanded);
    }

    fn generate_with_cached_type(&self, cached_type: &CachedType, tokens: &mut TokenStream) {
        let name = &self.source.ident;
        let cached_name = cached_type.ident();
        let (impl_generics, ty_generics, where_clause) = self.source.generics.split_for_impl();

        // All fields are copied through — Cached type has every field.
        // Skipped fields are handled by custom Clone (defaults on clone)
        // and serde (skip on serialization).
        let field_idents: Vec<_> = self
            .source
            .fields()
            .map(|f| f.ident.as_ref().expect("named field"))
            .collect();

        let expanded = quote! {
            impl #impl_generics hitbox::CacheableResponse for #name #ty_generics #where_clause {
                type Cached = #cached_name #ty_generics;
                type Subject = Self;
                type IntoCachedFuture = std::future::Ready<hitbox::CachePolicy<Self::Cached, Self>>;
                type FromCachedFuture = std::future::Ready<Self>;

                async fn cache_policy<__P, __TE>(
                    self,
                    predicates: __P,
                    tag_extractor: ::std::option::Option<__TE>,
                    config: &hitbox::EntityPolicyConfig,
                ) -> (
                    hitbox::ResponseCachePolicy<Self>,
                    ::std::vec::Vec<hitbox::tag::CacheTag>,
                )
                where
                    __P: hitbox::predicate::Predicate<Subject = Self::Subject> + Send + Sync,
                    __TE: hitbox::tag::TagExtractor<Subject = Self::Subject> + Send + Sync,
                {
                    match predicates.check(self).await {
                        hitbox::predicate::PredicateResult::Cacheable(data) => {
                            let (data, tags) = match tag_extractor {
                                ::std::option::Option::Some(__te) => {
                                    __te.extract_tags(data).await
                                }
                                ::std::option::Option::None => {
                                    (data, ::std::vec::Vec::new())
                                }
                            };
                            let cached = #cached_name {
                                #(#field_idents: data.#field_idents,)*
                            };
                            (
                                hitbox::CachePolicy::Cacheable(
                                    hitbox::CacheValue::from_config(cached, config),
                                ),
                                tags,
                            )
                        }
                        hitbox::predicate::PredicateResult::NonCacheable(data) => {
                            (hitbox::CachePolicy::NonCacheable(data), ::std::vec::Vec::new())
                        }
                    }
                }

                fn into_cached(self) -> Self::IntoCachedFuture {
                    let cached = #cached_name {
                        #(#field_idents: self.#field_idents,)*
                    };
                    std::future::ready(hitbox::CachePolicy::Cacheable(cached))
                }

                fn from_cached(cached: Self::Cached) -> Self::FromCachedFuture {
                    std::future::ready(#name {
                        #(#field_idents: cached.#field_idents,)*
                    })
                }
            }
        };

        tokens.extend(expanded);
    }
}

impl<'a> ToTokens for CacheableResponseImpl<'a> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self.cached_type {
            Some(cached_type) => self.generate_with_cached_type(cached_type, tokens),
            None => self.generate_simple(tokens),
        }
    }
}
