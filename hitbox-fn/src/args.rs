//! Argument wrapper for function caching.

use hitbox::{
    CachePolicy, CacheableRequest, Extractor, KeyPart, Predicate, RequestCachePolicy,
    predicate::PredicateResult,
};

use crate::KeyExtract;

/// Wrapper for individual function arguments with metadata for cache key extraction.
///
/// This type wraps each function argument and carries metadata about how it should
/// be handled during cache key generation (e.g., whether to skip it).
///
/// # Example
///
/// ```ignore
/// use hitbox_fn::Arg;
///
/// // Argument included in cache key
/// let arg = Arg::new(42);
///
/// // Argument excluded from cache key
/// let skipped = Arg::skipped("request-id".to_string());
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Arg<T> {
    value: T,
    skip: bool,
}

impl<T> Arg<T> {
    /// Create a new argument that will be included in the cache key.
    pub fn new(value: T) -> Self {
        Self { value, skip: false }
    }

    /// Create a new argument that will be skipped from the cache key.
    pub fn skipped(value: T) -> Self {
        Self { value, skip: true }
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
        if self.skip {
            vec![]
        } else {
            self.value.extract()
        }
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

impl CacheableRequest for Args<()> {
    async fn cache_policy<P, E>(self, predicates: P, extractors: E) -> RequestCachePolicy<Self>
    where
        P: Predicate<Subject = Self> + Send + Sync,
        E: Extractor<Subject = Self> + Send + Sync,
    {
        match predicates.check(self).await {
            PredicateResult::Cacheable(subject) => {
                let (subject, key) = extractors.get(subject).await.into_cache_key();
                CachePolicy::Cacheable(hitbox::CacheablePolicyData::new(key, subject))
            }
            PredicateResult::NonCacheable(subject) => CachePolicy::NonCacheable(subject),
        }
    }
}

impl<T0> CacheableRequest for Args<(T0,)>
where
    T0: Send + Sync + 'static,
{
    async fn cache_policy<P, E>(self, predicates: P, extractors: E) -> RequestCachePolicy<Self>
    where
        P: Predicate<Subject = Self> + Send + Sync,
        E: Extractor<Subject = Self> + Send + Sync,
    {
        match predicates.check(self).await {
            PredicateResult::Cacheable(subject) => {
                let (subject, key) = extractors.get(subject).await.into_cache_key();
                CachePolicy::Cacheable(hitbox::CacheablePolicyData::new(key, subject))
            }
            PredicateResult::NonCacheable(subject) => CachePolicy::NonCacheable(subject),
        }
    }
}

impl<T0, T1> CacheableRequest for Args<(T0, T1)>
where
    T0: Send + Sync + 'static,
    T1: Send + Sync + 'static,
{
    async fn cache_policy<P, E>(self, predicates: P, extractors: E) -> RequestCachePolicy<Self>
    where
        P: Predicate<Subject = Self> + Send + Sync,
        E: Extractor<Subject = Self> + Send + Sync,
    {
        match predicates.check(self).await {
            PredicateResult::Cacheable(subject) => {
                let (subject, key) = extractors.get(subject).await.into_cache_key();
                CachePolicy::Cacheable(hitbox::CacheablePolicyData::new(key, subject))
            }
            PredicateResult::NonCacheable(subject) => CachePolicy::NonCacheable(subject),
        }
    }
}

impl<T0, T1, T2> CacheableRequest for Args<(T0, T1, T2)>
where
    T0: Send + Sync + 'static,
    T1: Send + Sync + 'static,
    T2: Send + Sync + 'static,
{
    async fn cache_policy<P, E>(self, predicates: P, extractors: E) -> RequestCachePolicy<Self>
    where
        P: Predicate<Subject = Self> + Send + Sync,
        E: Extractor<Subject = Self> + Send + Sync,
    {
        match predicates.check(self).await {
            PredicateResult::Cacheable(subject) => {
                let (subject, key) = extractors.get(subject).await.into_cache_key();
                CachePolicy::Cacheable(hitbox::CacheablePolicyData::new(key, subject))
            }
            PredicateResult::NonCacheable(subject) => CachePolicy::NonCacheable(subject),
        }
    }
}

impl<T0, T1, T2, T3> CacheableRequest for Args<(T0, T1, T2, T3)>
where
    T0: Send + Sync + 'static,
    T1: Send + Sync + 'static,
    T2: Send + Sync + 'static,
    T3: Send + Sync + 'static,
{
    async fn cache_policy<P, E>(self, predicates: P, extractors: E) -> RequestCachePolicy<Self>
    where
        P: Predicate<Subject = Self> + Send + Sync,
        E: Extractor<Subject = Self> + Send + Sync,
    {
        match predicates.check(self).await {
            PredicateResult::Cacheable(subject) => {
                let (subject, key) = extractors.get(subject).await.into_cache_key();
                CachePolicy::Cacheable(hitbox::CacheablePolicyData::new(key, subject))
            }
            PredicateResult::NonCacheable(subject) => CachePolicy::NonCacheable(subject),
        }
    }
}

// Additional arities can be added via macro if needed
