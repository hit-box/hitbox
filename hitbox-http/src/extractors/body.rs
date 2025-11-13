use std::fmt::Debug;

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};
use hyper::body::Body as HttpBody;
use jaq_core::{
    self, Ctx, RcIter,
    load::{Arena, File, Loader},
};
use jaq_json::{self, Val};
use serde_json::Value;

use crate::CacheableHttpRequest;

#[derive(Debug)]
pub struct Body<E> {
    inner: E,
    expression: String,
}

impl<E> Body<E> {
    pub fn new(inner: E, expression: String) -> Self {
        Self { inner, expression }
    }
}

pub trait BodyExtractor: Sized {
    fn body(self, expression: String) -> Body<Self>;
}

impl<E> BodyExtractor for E
where
    E: Extractor,
{
    fn body(self, expression: String) -> Body<Self> {
        Body {
            inner: self,
            expression,
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
        Ok(values) if values.eq(&vec![Val::Null]) => Some(Value::Null),
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
impl<ReqBody, E> Extractor for Body<E>
where
    ReqBody: HttpBody + Send + 'static,
    ReqBody::Error: Debug + Send,
    ReqBody::Data: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let (parts, body) = subject.into_parts();

        // Collect body, handling errors
        let payload = match body.collect().await {
            Ok(bytes) => bytes,
            Err(error_body) => {
                // If collection fails, return early with None for this key part
                let request = CacheableHttpRequest::from_request(http::Request::from_parts(
                    parts, error_body,
                ));
                let mut key_parts = self.inner.get(request).await;
                key_parts.push(KeyPart::new(self.expression.clone(), None::<String>));
                return key_parts;
            }
        };

        let body_str = String::from_utf8_lossy(&payload);
        let json_value = serde_json::from_str(&body_str).unwrap_or(Value::Null);

        let found_value = apply(&self.expression, json_value);

        // Convert the extracted value to a string for the cache key
        // Null values are represented as None
        let value_string = found_value.and_then(|v| match v {
            Value::Null => None,
            Value::String(s) => Some(s),
            other => Some(other.to_string()),
        });

        let body = crate::BufferedBody::Complete(Some(payload));
        let request = CacheableHttpRequest::from_request(http::Request::from_parts(parts, body));

        let mut key_parts = self.inner.get(request).await;
        key_parts.push(KeyPart::new(self.expression.clone(), value_string));
        key_parts
    }
}
