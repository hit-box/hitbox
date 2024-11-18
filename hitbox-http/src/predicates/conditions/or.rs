use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::predicate::PredicateResult;
use hitbox::Predicate;

pub struct Or<L, R> {
    left: L,
    right: R,
}

#[async_trait]
impl<L, R, ReqBody> Predicate for Or<L, R>
where
    ReqBody: Send + 'static,
    L: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
    R: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = CacheableHttpRequest<ReqBody>;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        let left = self.left.check(request).await;
        match left {
            PredicateResult::Cacheable(request) => PredicateResult::Cacheable(request),
            PredicateResult::NonCacheable(request) => match self.right.check(request).await {
                PredicateResult::Cacheable(request) => PredicateResult::Cacheable(request),
                PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
            },
        }
    }
}

pub trait OrPredicate: Sized {
    fn or<P: Predicate>(self, predicate: P) -> Or<Self, P>;
}

impl<T> OrPredicate for T
where
    T: Predicate,
{
    fn or<P>(self, predicate: P) -> Or<Self, P> {
        Or {
            left: self,
            right: predicate,
        }
    }
}
