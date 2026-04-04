//! Argument wrapper for function caching.

use std::future::Future;
use std::pin::Pin;

use hitbox::{
    CachePolicy, CacheableRequest, Extractor, KeyPart, Predicate, RequestCachePolicy,
    predicate::PredicateResult,
};

use crate::KeyExtract;

/// Wrapper for function arguments included in cache key extraction.
///
/// This type wraps arguments that should contribute to the cache key.
/// The inner type must implement `KeyExtract`.
///
/// The parameter name is used to disambiguate cache keys:
/// - Single inner `KeyPart` → key is replaced with the parameter name
/// - Multiple inner `KeyParts` → each key is prefixed with `"param_name."`
#[derive(Clone, Debug)]
pub struct Arg<T> {
    name: &'static str,
    value: T,
}

impl<T> Arg<T> {
    /// Create a new named argument that will be included in the cache key.
    pub fn new(name: &'static str, value: T) -> Self {
        Self { name, value }
    }

    /// Get a reference to the inner value.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Unwrap and return the inner value.
    pub fn into_value(self) -> T {
        self.value
    }
}

impl<T: KeyExtract> KeyExtract for Arg<T> {
    fn extract(&self) -> Vec<KeyPart> {
        let inner = self.value.extract();
        if inner.len() == 1 {
            inner.into_iter().map(|p| p.with_key(self.name)).collect()
        } else {
            inner.into_iter().map(|p| p.prefixed(self.name)).collect()
        }
    }
}

/// Wrapper for function arguments excluded from cache key extraction.
///
/// This type wraps arguments that should NOT contribute to the cache key.
/// The inner type does NOT need to implement `KeyExtract`.
///
/// Useful for skipping types like database connections, request contexts,
/// or other non-cacheable dependencies.
#[derive(Clone, Debug)]
pub struct Skipped<T>(T);

impl<T> Skipped<T> {
    /// Create a new skipped argument that will be excluded from the cache key.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Get a reference to the inner value.
    pub fn value(&self) -> &T {
        &self.0
    }

    /// Unwrap and return the inner value.
    pub fn into_value(self) -> T {
        self.0
    }
}

impl<T> KeyExtract for Skipped<T> {
    fn extract(&self) -> Vec<KeyPart> {
        vec![]
    }
}

/// Wrapper around tuple to satisfy orphan rule.
///
/// This wrapper enables implementing hitbox traits for tuples of function arguments.
/// Without this wrapper, we couldn't implement foreign traits (`CacheableRequest`, etc.)
/// for foreign types (tuples).
#[derive(Clone, Debug)]
pub struct Args<T>(pub T);

impl<T> Args<T> {
    /// Create a new Args wrapper.
    pub fn new(inner: T) -> Self {
        Self(inner)
    }

    /// Unwrap and return the inner value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

// CacheableRequest implementations for Args<tuples>

macro_rules! impl_cacheable_request_for_args {
    () => {
        impl CacheableRequest for Args<()> {
            type CachePolicyFuture<'a, P, E>
                = Pin<Box<dyn Future<Output = RequestCachePolicy<Self>> + Send + 'a>>
            where
                Self: 'a,
                P: Predicate<Subject = Self> + Send + Sync + 'a,
                E: Extractor<Subject = Self> + Send + Sync + 'a;

            fn cache_policy<'a, P, E>(
                self,
                predicates: P,
                extractors: E,
            ) -> Self::CachePolicyFuture<'a, P, E>
            where
                Self: 'a,
                P: Predicate<Subject = Self> + Send + Sync + 'a,
                E: Extractor<Subject = Self> + Send + Sync + 'a,
            {
                Box::pin(async move {
                    let mut ctx = hitbox::EvalContext::new();
                    match predicates.check(self, &mut ctx).await {
                        PredicateResult::Cacheable(subject) => {
                            let (subject, key) = extractors.get(subject, &mut ctx).await.into_cache_key();
                            CachePolicy::Cacheable(hitbox::CacheablePolicyData::new(key, subject))
                        }
                        PredicateResult::NonCacheable(subject) => CachePolicy::NonCacheable(subject),
                    }
                })
            }
        }
    };
    ($($T:ident),+) => {
        impl<$($T: Send + Sync),+> CacheableRequest for Args<($($T,)+)> {
            type CachePolicyFuture<'a, P, E>
                = Pin<Box<dyn Future<Output = RequestCachePolicy<Self>> + Send + 'a>>
            where
                Self: 'a,
                P: Predicate<Subject = Self> + Send + Sync + 'a,
                E: Extractor<Subject = Self> + Send + Sync + 'a;

            fn cache_policy<'a, P, E>(
                self,
                predicates: P,
                extractors: E,
            ) -> Self::CachePolicyFuture<'a, P, E>
            where
                Self: 'a,
                P: Predicate<Subject = Self> + Send + Sync + 'a,
                E: Extractor<Subject = Self> + Send + Sync + 'a,
            {
                Box::pin(async move {
                    let mut ctx = hitbox::EvalContext::new();
                    match predicates.check(self, &mut ctx).await {
                        PredicateResult::Cacheable(subject) => {
                            let (subject, key) = extractors.get(subject, &mut ctx).await.into_cache_key();
                            CachePolicy::Cacheable(hitbox::CacheablePolicyData::new(key, subject))
                        }
                        PredicateResult::NonCacheable(subject) => CachePolicy::NonCacheable(subject),
                    }
                })
            }
        }
    };
}

impl_cacheable_request_for_args!();
impl_cacheable_request_for_args!(T0);
impl_cacheable_request_for_args!(T0, T1);
impl_cacheable_request_for_args!(T0, T1, T2);
impl_cacheable_request_for_args!(T0, T1, T2, T3);
impl_cacheable_request_for_args!(T0, T1, T2, T3, T4);
impl_cacheable_request_for_args!(T0, T1, T2, T3, T4, T5);
impl_cacheable_request_for_args!(T0, T1, T2, T3, T4, T5, T6);
impl_cacheable_request_for_args!(T0, T1, T2, T3, T4, T5, T6, T7);
impl_cacheable_request_for_args!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
impl_cacheable_request_for_args!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_cacheable_request_for_args!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_cacheable_request_for_args!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
