use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};
use http::{HeaderName, HeaderValue};

#[derive(Debug)]
pub enum Operation {
    Eq(HeaderName, HeaderValue),
    Exist(HeaderName),
    In(HeaderName, Vec<HeaderValue>),
}

#[derive(Debug)]
pub struct Header<P> {
    pub operation: Operation,
    inner: P,
}

impl<P> Header<P> {
    pub fn new(inner: P, operation: Operation) -> Self {
        Self { operation, inner }
    }
}

pub trait HeaderPredicate: Sized {
    fn header(self, operation: Operation) -> Header<Self>;
}

impl<P> HeaderPredicate for P
where
    P: Predicate,
{
    fn header(self, operation: Operation) -> Header<Self> {
        Header {
            operation,
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for Header<P>
where
    ReqBody: Send + 'static,
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = P::Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(request).await {
            PredicateResult::Cacheable(request) => {
                let is_cacheable = match &self.operation {
                    Operation::Eq(name, value) => request
                        .parts()
                        .headers
                        .get(name)
                        .is_some_and(|v| v.eq(value)),
                    Operation::Exist(name) => request.parts().headers.get(name).is_some(),
                    Operation::In(name, values) => request
                        .parts()
                        .headers
                        .get(name)
                        .is_some_and(|v| values.contains(v)),
                };
                if is_cacheable {
                    PredicateResult::Cacheable(request)
                } else {
                    PredicateResult::NonCacheable(request)
                }
            }
            PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
        }
    }
}
