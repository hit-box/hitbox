use std::fmt::Debug;

use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};
use http::Request;
use hyper::body::Body as HttpBody;
use serde_json::Value;

use crate::{CacheableHttpRequest, FromBytes, MAX_BODY_SIZE, ParsingType, body_processing};

#[derive(Debug)]
pub enum Operation {
    Eq(Value),
    Exist,
    In(Vec<Value>), // TODO: Add key-value pairs
}

#[derive(Debug)]
pub struct Body<P> {
    operation: Operation,
    parsing_type: ParsingType,
    expression: String,
    inner: P,
}

pub trait BodyPredicate: Sized {
    fn body(
        self,
        parsing_type: ParsingType,
        expression: String,
        operation: Operation,
    ) -> Body<Self>;
}

impl<P> BodyPredicate for P
where
    P: Predicate,
{
    fn body(
        self,
        parsing_type: ParsingType,
        expression: String,
        operation: Operation,
    ) -> Body<Self> {
        Body {
            operation,
            parsing_type,
            expression,
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for Body<P>
where
    ReqBody: HttpBody + Send + 'static,
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
    ReqBody::Error: Debug,
    ReqBody::Data: Send,
    ReqBody: FromBytes,
{
    type Subject = P::Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(request).await {
            PredicateResult::Cacheable(request) => {
                let (parts, body) = request.into_parts();
                let payload = body_processing::collect_body(body, MAX_BODY_SIZE)
                    .await
                    .unwrap();
                let json_value = body_processing::parse_body(&payload, &self.parsing_type).unwrap();
                let found_value =
                    body_processing::apply_jq_expression(&self.expression, json_value).unwrap();

                let is_cacheable = match &self.operation {
                    Operation::Eq(expected) => {
                        found_value.map(|v| v.eq(expected)).unwrap_or_default()
                    }
                    Operation::Exist => found_value.is_some(),
                    Operation::In(values) => {
                        found_value.map(|v| values.contains(&v)).unwrap_or_default()
                    }
                };

                let request = Request::from_parts(parts, ReqBody::from_bytes(payload));
                if is_cacheable {
                    PredicateResult::Cacheable(CacheableHttpRequest::from_request(request))
                } else {
                    PredicateResult::NonCacheable(CacheableHttpRequest::from_request(request))
                }
            }
            PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
        }
    }
}
