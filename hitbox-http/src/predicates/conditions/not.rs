use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::Predicate;
use hitbox::predicate::PredicateResult;

#[derive(Debug)]
pub struct Not<T> {
    predicate: T,
}

impl<T> Not<T> {
    pub fn new(predicate: T) -> Self {
        Self { predicate }
    }
}

#[async_trait]
impl<T, ReqBody> Predicate for Not<T>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    T: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = CacheableHttpRequest<ReqBody>;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.predicate.check(request).await {
            PredicateResult::Cacheable(request) => PredicateResult::NonCacheable(request),
            PredicateResult::NonCacheable(request) => PredicateResult::Cacheable(request),
        }
    }
}

pub trait NotPredicate: Sized {
    fn not<P: Predicate>(self, predicate: P) -> Not<P>;
}

impl<T> NotPredicate for T
where
    T: Predicate,
{
    fn not<P>(self, predicate: P) -> Not<P> {
        Not { predicate }
    }
}
