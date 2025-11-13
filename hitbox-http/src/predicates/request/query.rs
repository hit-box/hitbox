use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};

#[derive(Debug)]
pub enum Operation {
    Eq(String, String),
    Exist(String),
    In(String, Vec<String>),
}

#[derive(Debug)]
pub struct Query<P> {
    pub operation: Operation,
    inner: P,
}

impl<P> Query<P> {
    pub fn new(inner: P, operation: Operation) -> Self {
        Self { operation, inner }
    }
}

pub trait QueryPredicate: Sized {
    fn query(self, operation: Operation) -> Query<Self>;
}

impl<P> QueryPredicate for P
where
    P: Predicate,
{
    fn query(self, operation: Operation) -> Query<Self> {
        Query {
            operation,
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for Query<P>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = P::Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(request).await {
            PredicateResult::Cacheable(request) => {
                let is_cacheable = match request.parts().uri.query().map(crate::query::parse) {
                    Some(query_map) => match &self.operation {
                        Operation::Eq(name, value) => query_map
                            .get(name)
                            .map(|v| v.contains(value))
                            .unwrap_or_default(),
                        Operation::Exist(name) => {
                            query_map.get(name).map(|_| true).unwrap_or_default()
                        }
                        Operation::In(name, values) => query_map
                            .get(name)
                            .and_then(|value| values.iter().find(|v| value.contains(v)))
                            .map(|_| true)
                            .unwrap_or_default(),
                    },
                    None => false,
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
