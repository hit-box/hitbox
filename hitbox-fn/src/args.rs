//! Argument wrapper for function caching.

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
/// # Example
///
/// ```ignore
/// use hitbox_fn::Arg;
///
/// // Argument included in cache key
/// let arg = Arg::new(42);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Arg<T>(T);

impl<T> Arg<T> {
    /// Create a new argument that will be included in the cache key.
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

impl<T: KeyExtract> KeyExtract for Arg<T> {
    fn extract(&self) -> Vec<KeyPart> {
        self.0.extract()
    }
}

/// Wrapper for function arguments excluded from cache key extraction.
///
/// This type wraps arguments that should NOT contribute to the cache key.
/// The inner type does NOT need to implement `KeyExtract`.
///
/// Useful for skipping types like database connections, request contexts,
/// or other non-cacheable dependencies.
///
/// # Example
///
/// ```ignore
/// use hitbox_fn::Skipped;
///
/// // Argument excluded from cache key (no KeyExtract bound needed)
/// let skipped = Skipped::new(db_connection);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
///
/// # Example
///
/// ```ignore
/// use hitbox_fn::Args;
///
/// // Wrap function arguments
/// let args = Args((user_id, tenant_id));
///
/// // Access inner tuple
/// let (user_id, tenant_id) = args.0;
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

use std::pin::Pin;
use std::future::Future;

impl CacheableRequest for Args<()> {
    type CachePolicyFuture<'a, P, E> = Pin<Box<dyn Future<Output = RequestCachePolicy<Self>> + Send + 'a>>
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a;

    fn cache_policy<'a, P, E>(self, predicates: P, extractors: E) -> Self::CachePolicyFuture<'a, P, E>
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a,
    {
        Box::pin(async move {
            match predicates.check(self).await {
                PredicateResult::Cacheable(subject) => {
                    let (subject, key) = extractors.get(subject).await.into_cache_key();
                    CachePolicy::Cacheable(hitbox::CacheablePolicyData::new(key, subject))
                }
                PredicateResult::NonCacheable(subject) => CachePolicy::NonCacheable(subject),
            }
        })
    }
}

impl<T0> CacheableRequest for Args<(T0,)>
where
    T0: Send + Sync,
{
    type CachePolicyFuture<'a, P, E> = Pin<Box<dyn Future<Output = RequestCachePolicy<Self>> + Send + 'a>>
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a;

    fn cache_policy<'a, P, E>(self, predicates: P, extractors: E) -> Self::CachePolicyFuture<'a, P, E>
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a,
    {
        Box::pin(async move {
            match predicates.check(self).await {
                PredicateResult::Cacheable(subject) => {
                    let (subject, key) = extractors.get(subject).await.into_cache_key();
                    CachePolicy::Cacheable(hitbox::CacheablePolicyData::new(key, subject))
                }
                PredicateResult::NonCacheable(subject) => CachePolicy::NonCacheable(subject),
            }
        })
    }
}

impl<T0, T1> CacheableRequest for Args<(T0, T1)>
where
    T0: Send + Sync,
    T1: Send + Sync,
{
    type CachePolicyFuture<'a, P, E> = Pin<Box<dyn Future<Output = RequestCachePolicy<Self>> + Send + 'a>>
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a;

    fn cache_policy<'a, P, E>(self, predicates: P, extractors: E) -> Self::CachePolicyFuture<'a, P, E>
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a,
    {
        Box::pin(async move {
            match predicates.check(self).await {
                PredicateResult::Cacheable(subject) => {
                    let (subject, key) = extractors.get(subject).await.into_cache_key();
                    CachePolicy::Cacheable(hitbox::CacheablePolicyData::new(key, subject))
                }
                PredicateResult::NonCacheable(subject) => CachePolicy::NonCacheable(subject),
            }
        })
    }
}

impl<T0, T1, T2> CacheableRequest for Args<(T0, T1, T2)>
where
    T0: Send + Sync,
    T1: Send + Sync,
    T2: Send + Sync,
{
    type CachePolicyFuture<'a, P, E> = Pin<Box<dyn Future<Output = RequestCachePolicy<Self>> + Send + 'a>>
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a;

    fn cache_policy<'a, P, E>(self, predicates: P, extractors: E) -> Self::CachePolicyFuture<'a, P, E>
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a,
    {
        Box::pin(async move {
            match predicates.check(self).await {
                PredicateResult::Cacheable(subject) => {
                    let (subject, key) = extractors.get(subject).await.into_cache_key();
                    CachePolicy::Cacheable(hitbox::CacheablePolicyData::new(key, subject))
                }
                PredicateResult::NonCacheable(subject) => CachePolicy::NonCacheable(subject),
            }
        })
    }
}

impl<T0, T1, T2, T3> CacheableRequest for Args<(T0, T1, T2, T3)>
where
    T0: Send + Sync,
    T1: Send + Sync,
    T2: Send + Sync,
    T3: Send + Sync,
{
    type CachePolicyFuture<'a, P, E> = Pin<Box<dyn Future<Output = RequestCachePolicy<Self>> + Send + 'a>>
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a;

    fn cache_policy<'a, P, E>(self, predicates: P, extractors: E) -> Self::CachePolicyFuture<'a, P, E>
    where
        Self: 'a,
        P: Predicate<Subject = Self> + Send + Sync + 'a,
        E: Extractor<Subject = Self> + Send + Sync + 'a,
    {
        Box::pin(async move {
            match predicates.check(self).await {
                PredicateResult::Cacheable(subject) => {
                    let (subject, key) = extractors.get(subject).await.into_cache_key();
                    CachePolicy::Cacheable(hitbox::CacheablePolicyData::new(key, subject))
                }
                PredicateResult::NonCacheable(subject) => CachePolicy::NonCacheable(subject),
            }
        })
    }
}

// Additional arities can be added via macro if needed
