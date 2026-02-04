//! Argument wrapper for function caching.

use hitbox::{
    CachePolicy, CacheableRequest, Extractor, Predicate, RequestCachePolicy,
    predicate::PredicateResult,
};

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
