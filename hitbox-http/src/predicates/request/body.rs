use std::fmt::Debug;

use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};
use http::Request;
use hyper::body::Body as HttpBody;
use jaq_core::{
    self,
    load::{Arena, File, Loader},
    Ctx, RcIter,
};
use jaq_json::{self, Val};
use prost_reflect::{DynamicMessage, MessageDescriptor, SerializeOptions};
use serde_json::Value;

use crate::{CacheableHttpRequest, FromBytes};

pub enum Operation {
    Eq(Value),
    Exist,
    In(Vec<Value>), // TODO: Add key-value pairs
}

pub enum ParsingType {
    Jq,
    ProtoBuf(MessageDescriptor),
}

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

fn apply(expression: &str, input: Value) -> Option<Value> {
    // TODO: Handle the errors.
    let program = File {
        code: expression,
        path: (),
    };
    let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()));
    let arena = Arena::default();
    let modules = loader.load(&arena, program).unwrap();
    let filter = jaq_core::Compiler::default()
        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
        .compile(modules)
        .unwrap();
    let inputs = RcIter::new(core::iter::empty());
    let out = filter.run((Ctx::new([], &inputs), Val::from(input)));
    let results: Result<Vec<_>, _> = out.collect();
    match results {
        Ok(values) if values.eq(&vec![Val::Null]) => None,
        Ok(values) if !values.is_empty() => {
            let values: Vec<Value> = values.into_iter().map(|v| v.into()).collect();
            if values.len() == 1 {
                Some(values.into_iter().next().unwrap())
            } else {
                Some(Value::Array(values))
            }
        }
        _ => None,
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
                use http_body_util::BodyExt;
                let payload = body.collect().await.unwrap().to_bytes();
                // let payload = to_bytes(body).await.unwrap();
                let body_str = String::from_utf8_lossy(&payload);
                let json_value = match &self.parsing_type {
                    ParsingType::Jq => serde_json::from_str(&body_str).unwrap_or(Value::Null),
                    ParsingType::ProtoBuf(message) => {
                        let dynamic_message =
                            DynamicMessage::decode(message.clone(), payload.as_ref()).unwrap();
                        let mut serializer = serde_json::Serializer::new(vec![]);
                        let options = SerializeOptions::new().skip_default_fields(false);
                        dynamic_message
                            .serialize_with_options(&mut serializer, &options)
                            .unwrap();
                        serde_json::from_slice(&serializer.into_inner()).unwrap()
                    }
                };
                let found_value = apply(&self.expression, json_value);

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
