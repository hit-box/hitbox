use std::fmt::Debug;

use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};
use hyper::body::Body as HttpBody;
use jaq_core::{
    self, Ctx, RcIter,
    load::{Arena, File, Loader},
};
use jaq_json::{self, Val};
use prost_reflect::{DynamicMessage, MessageDescriptor, SerializeOptions};
use serde_json::Value;

use crate::FromBytes;

/// Trait that abstracts over HTTP messages (requests and responses) to enable
/// generic body predicate processing by partitioning them into parts and body.
pub trait HttpPartition: Sized {
    type Body: HttpBody + FromBytes;
    type Parts: Send;

    /// Deconstruct the message into its parts and body.
    fn into_parts_and_body(self) -> (Self::Parts, Self::Body);

    /// Reconstruct the message from parts and body.
    fn from_parts_and_body(parts: Self::Parts, body: Self::Body) -> Self;
}

#[derive(Debug)]
pub enum Operation {
    Eq(Value),
    Exist,
    In(Vec<Value>), // TODO: Add key-value pairs
}

#[derive(Debug)]
pub enum ParsingType {
    Jq,
    ProtoBuf(MessageDescriptor),
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
impl<P, M> Predicate for Body<P>
where
    M: HttpPartition + Send + 'static,
    M::Body: Send + 'static,
    <M::Body as HttpBody>::Error: Debug,
    <M::Body as HttpBody>::Data: Send,
    P: Predicate<Subject = M> + Send + Sync,
{
    type Subject = P::Subject;

    async fn check(&self, message: Self::Subject) -> Result<PredicateResult<Self::Subject>, hitbox::PredicateError> {
        use http_body_util::BodyExt;

        match self.inner.check(message).await? {
            PredicateResult::Cacheable(message) => {
                let (parts, body) = message.into_parts_and_body();
                let payload = match body.collect().await {
                    Ok(collected) => collected.to_bytes(),
                    Err(e) => {
                        // Convert error to string since body error types don't always implement std::error::Error
                        let error_msg = format!("Failed to collect HTTP body: {:?}", e);
                        let io_error = std::io::Error::new(std::io::ErrorKind::Other, error_msg);
                        return Err(hitbox::PredicateError::Internal(Box::new(io_error)));
                    }
                };
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

                let message = M::from_parts_and_body(parts, M::Body::from_bytes(payload));
                if is_cacheable {
                    Ok(PredicateResult::Cacheable(message))
                } else {
                    Ok(PredicateResult::NonCacheable(message))
                }
            }
            PredicateResult::NonCacheable(message) => Ok(PredicateResult::NonCacheable(message)),
        }
    }
}
