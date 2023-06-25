use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::predicates::{Operation, Predicate, PredicateResult};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum QsValue {
    Scalar(String),
    Array(Vec<String>),
}

pub struct Query {
    pub name: String,
    pub value: QsValue,
    pub operation: Operation,
}

fn parse_query(value: &str) -> HashMap<String, QsValue> {
    serde_qs::from_str(value).unwrap()
}

#[async_trait]
impl Predicate<CacheableHttpRequest> for Query {
    async fn check(&self, request: CacheableHttpRequest) -> PredicateResult<CacheableHttpRequest> {
        let op = match self.operation {
            Operation::Eq => QsValue::eq,
            Operation::In => unimplemented!(),
        };
        match request.parts().uri.query() {
            Some(query_string) => match parse_query(query_string).get(&self.name) {
                Some(value) if op(value, &self.value) => PredicateResult::Cacheable(request),
                _ => PredicateResult::NonCacheable(request),
            },
            None => PredicateResult::NonCacheable(request),
        }
    }
}
