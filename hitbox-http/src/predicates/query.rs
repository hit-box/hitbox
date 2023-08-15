use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::predicates::{Operation, Predicate, PredicateResult};

pub struct Query<P> {
    pub name: String,
    pub value: crate::query::Value,
    pub operation: Operation,
    inner: P,
}

impl<P> Query<P> {
    pub fn new(name: String, value: crate::query::Value, operation: Operation, inner: P) -> Self {
        Self {
            name,
            value,
            operation,
            inner,
        }
    }
}

pub trait QueryPredicate: Sized {
    fn query(self, name: String, value: String) -> Query<Self>;
}

impl<P> QueryPredicate for P
where
    P: Predicate,
{
    fn query(self, name: String, value: String) -> Query<P> {
        Query {
            name,
            value: crate::query::Value::Scalar(value),
            operation: Operation::Eq,
            inner: self,
        }
    }
}

#[async_trait]
impl<ReqBody, P> Predicate for Query<P>
where
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
    ReqBody: Send + 'static,
{
    type Subject = P::Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(request).await {
            PredicateResult::Cacheable(request) => {
                let op = match self.operation {
                    Operation::Eq => crate::query::Value::eq,
                    Operation::In => unimplemented!(),
                };
                match request.parts().uri.query() {
                    Some(query_string) => match crate::query::parse(query_string).get(&self.name) {
                        Some(value) if op(value, &self.value) => {
                            PredicateResult::Cacheable(request)
                        }
                        _ => PredicateResult::NonCacheable(request),
                    },
                    None => PredicateResult::NonCacheable(request),
                }
            }
            PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
        }
    }
}
