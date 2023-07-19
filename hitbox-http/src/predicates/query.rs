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

pub struct Query<P> {
    pub name: String,
    pub value: QsValue,
    pub operation: Operation,
    inner: P,
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
