use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};

pub enum Operation {
    Eq(String, String),
    Exist(String),
    In(String, Vec<String>),
}

pub struct Query<P> {
    pub operation: Operation,
    inner: P,
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
    ReqBody: Send + 'static,
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
                            .map(|value| values.iter().find(|v| value.contains(v)))
                            .flatten()
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
