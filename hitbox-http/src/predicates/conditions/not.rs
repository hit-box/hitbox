use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::predicate::PredicateResult;
use hitbox::Predicate;

#[derive(Debug)]
pub struct Not<P, N> {
    prev: P,
    next: N,
}

#[async_trait]
impl<P, N, ReqBody> Predicate for Not<P, N>
where
    ReqBody: Send + 'static,
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
    N: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = CacheableHttpRequest<ReqBody>;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.prev.check(request).await {
            PredicateResult::Cacheable(request) => match self.next.check(request).await {
                PredicateResult::Cacheable(request) => PredicateResult::NonCacheable(request),
                PredicateResult::NonCacheable(request) => PredicateResult::Cacheable(request),
            },
            PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
        }
    }
}

pub trait NotPredicate: Sized {
    fn not<P: Predicate>(self, predicate: P) -> Not<Self, P>;
}

impl<T> NotPredicate for T
where
    T: Predicate,
{
    fn not<P>(self, predicate: P) -> Not<Self, P> {
        Not {
            prev: self,
            next: predicate,
        }
    }
}
