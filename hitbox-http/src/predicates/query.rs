use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::predicates::{Operation, Predicate, PredicateResult};
use serde::Deserialize;
use std::{collections::HashMap, marker::PhantomData};

#[derive(Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum QsValue {
    Scalar(String),
    Array(Vec<String>),
}

pub struct Query<P: ?Sized> {
    pub name: String,
    pub value: QsValue,
    pub operation: Operation,
    inner: P,
}

pub trait QueryPredicate<T> {
    fn query(self, name: String, value: String) -> Query<Self>;
}

impl<P, T> QueryPredicate<T> for P
where
    P: Predicate<T>,
{
    fn query(self, name: String, value: String) -> Query<P> {
        Query {
            name,
            value: QsValue::Scalar(value),
            operation: Operation::Eq,
            inner: self,
        }
    }
}

fn parse_query(value: &str) -> HashMap<String, QsValue> {
    serde_qs::from_str(value).unwrap()
}

#[async_trait]
impl<ReqBody, P> Predicate<CacheableHttpRequest<ReqBody>> for Query<P>
where
    P: Predicate<CacheableHttpRequest<ReqBody>> + Send + Sync,
    ReqBody: Send + 'static,
{
    async fn check(
        &self,
        request: CacheableHttpRequest<ReqBody>,
    ) -> PredicateResult<CacheableHttpRequest<ReqBody>> {
        match self.inner.check(request).await {
            PredicateResult::Cacheable(request) => {
                let op = match self.operation {
                    Operation::Eq => QsValue::eq,
                    Operation::In => unimplemented!(),
                };
                match request.parts().uri.query() {
                    Some(query_string) => match parse_query(query_string).get(&self.name) {
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
