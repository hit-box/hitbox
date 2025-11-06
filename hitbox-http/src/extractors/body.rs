use std::fmt::Debug;

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};
use hyper::body::Body as HttpBody;
use serde_json::Value;

use crate::{CacheableHttpRequest, FromBytes, MAX_BODY_SIZE, body_processing};

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

#[async_trait]
impl<ReqBody, E> Extractor for Body<E>
where
    ReqBody: HttpBody + FromBytes + Send + 'static,
    ReqBody::Error: Debug,
    ReqBody::Data: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let (parts, body) = subject.into_parts();
        let payload = body_processing::collect_body(body, MAX_BODY_SIZE)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&payload);
        let json_value = serde_json::from_str(&body_str).unwrap_or(Value::Null);

        let found_value =
            body_processing::apply_jq_expression(&self.expression, json_value).unwrap();

        // Convert the extracted value to a string for the cache key
        // Null values are represented as None
        let value_string = found_value.and_then(|v| match v {
            Value::Null => None,
            Value::String(s) => Some(s),
            other => Some(other.to_string()),
        });

        let request = CacheableHttpRequest::from_request(http::Request::from_parts(
            parts,
            ReqBody::from_bytes(payload),
        ));

        let mut key_parts = self.inner.get(request).await;
        key_parts.push(KeyPart::new(self.expression.clone(), value_string));
        key_parts
    }
}
