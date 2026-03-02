//! Runtime protobuf field extractor using `prost-reflect`.
//!
//! [`ProtoFieldExtractor`] extracts protobuf field values as cache key parts
//! using runtime reflection.

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};
use hyper::body::Body as HttpBody;
use prost_reflect::MessageDescriptor;
use tracing::warn;

use crate::decode::BytesExtractor;

/// Extracts a protobuf message field value as a cache key part.
///
/// Decodes the request body using runtime reflection and extracts the specified
/// field as `KeyPart::new(field_path, Some(value_string))`.
///
/// # Type Parameters
///
/// * `E` - The inner extractor in the chain
#[derive(Debug)]
pub struct ProtoFieldExtractor<E> {
    message_descriptor: MessageDescriptor,
    field_path: String,
    bytes_extractor: BytesExtractor,
    inner: E,
}

impl<S> ProtoFieldExtractor<hitbox_http::extractors::NeutralExtractor<S>> {
    /// Creates a new proto field extractor that starts an extractor chain.
    pub fn new(
        message_descriptor: MessageDescriptor,
        field_path: impl Into<String>,
        bytes_extractor: BytesExtractor,
    ) -> Self {
        ProtoFieldExtractor {
            message_descriptor,
            field_path: field_path.into(),
            bytes_extractor,
            inner: hitbox_http::extractors::NeutralExtractor::new(),
        }
    }
}

impl<E> ProtoFieldExtractor<E> {
    /// Creates a proto field extractor chained after an existing extractor.
    ///
    /// Unlike the [`ProtoFieldExtract`] extension trait, this does not require
    /// `E: Extractor` at construction time, making it suitable for use in
    /// convenience methods on protocol-specific extractors.
    pub fn after(
        inner: E,
        message_descriptor: MessageDescriptor,
        field_path: impl Into<String>,
        bytes_extractor: BytesExtractor,
    ) -> Self {
        ProtoFieldExtractor {
            message_descriptor,
            field_path: field_path.into(),
            bytes_extractor,
            inner,
        }
    }
}

/// Extension trait for adding proto field extraction to an extractor chain.
pub trait ProtoFieldExtract: Sized {
    /// Adds protobuf field extraction to this extractor chain.
    fn proto_field(
        self,
        message_descriptor: MessageDescriptor,
        field_path: impl Into<String>,
        bytes_extractor: BytesExtractor,
    ) -> ProtoFieldExtractor<Self>;
}

impl<E> ProtoFieldExtract for E
where
    E: Extractor,
{
    fn proto_field(
        self,
        message_descriptor: MessageDescriptor,
        field_path: impl Into<String>,
        bytes_extractor: BytesExtractor,
    ) -> ProtoFieldExtractor<Self> {
        ProtoFieldExtractor {
            message_descriptor,
            field_path: field_path.into(),
            bytes_extractor,
            inner: self,
        }
    }
}

/// Convert a prost-reflect Value to a string suitable for cache keys.
fn value_to_key_string(value: &prost_reflect::Value) -> String {
    match value {
        prost_reflect::Value::Bool(b) => b.to_string(),
        prost_reflect::Value::I32(n) => n.to_string(),
        prost_reflect::Value::I64(n) => n.to_string(),
        prost_reflect::Value::U32(n) => n.to_string(),
        prost_reflect::Value::U64(n) => n.to_string(),
        prost_reflect::Value::F32(n) => n.to_string(),
        prost_reflect::Value::F64(n) => n.to_string(),
        prost_reflect::Value::String(s) => s.clone(),
        prost_reflect::Value::Bytes(b) => hex::encode(b),
        prost_reflect::Value::EnumNumber(n) => n.to_string(),
        prost_reflect::Value::Message(_) => "<message>".to_string(),
        prost_reflect::Value::List(l) => format!("[{}items]", l.len()),
        prost_reflect::Value::Map(m) => format!("[{}entries]", m.len()),
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for ProtoFieldExtractor<E>
where
    ReqBody: HttpBody + Send + 'static,
    ReqBody::Error: Send,
    ReqBody::Data: Send,
    E: Extractor<Subject = hitbox_http::CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let (parts, body) = subject.into_parts();

        let collected = match body.collect().await {
            Ok(collected) => collected,
            Err(error_body) => {
                let request = hitbox_http::CacheableHttpRequest::from_request(
                    http::Request::from_parts(parts, error_body),
                );
                let mut key_parts = self.inner.get(request).await;
                key_parts.push(KeyPart::new(&self.field_path, None::<String>));
                return key_parts;
            }
        };

        let body_bytes = collected.data;
        let extracted = match crate::decode::decode_dynamic(
            &self.message_descriptor,
            &body_bytes,
            self.bytes_extractor,
        ) {
            Ok(msg) => match msg.get_field_by_name(&self.field_path) {
                Some(value) => {
                    let value_str = value_to_key_string(value.as_ref());
                    KeyPart::new(&self.field_path, Some(value_str))
                }
                None => {
                    warn!(field = %self.field_path, "ProtoFieldExtractor: field not found");
                    KeyPart::new(&self.field_path, None::<String>)
                }
            },
            Err(e) => {
                warn!(%e, "ProtoFieldExtractor: failed to decode protobuf");
                KeyPart::new(&self.field_path, None::<String>)
            }
        };

        let body = hitbox_http::BufferedBody::Complete {
            data: Some(body_bytes),
            trailers: collected.trailers,
        };
        let request =
            hitbox_http::CacheableHttpRequest::from_request(http::Request::from_parts(parts, body));

        let mut key_parts = self.inner.get(request).await;
        key_parts.push(extracted);
        key_parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decode::identity_bytes;
    use bytes::Bytes;
    use hitbox_http::CacheableHttpRequest;
    use http_body_util::Full;
    use prost::Message;
    use prost_reflect::{DynamicMessage, Value};

    fn test_setup() -> (crate::descriptor::ProtoDescriptors, MessageDescriptor) {
        let descriptors = crate::test_util::test_descriptors();
        let msg_desc = descriptors.get_message("test.GetUserRequest").unwrap();
        (descriptors, msg_desc)
    }

    fn encode_request(msg_desc: &MessageDescriptor, user_id: i64, name: &str) -> Vec<u8> {
        let mut msg = DynamicMessage::new(msg_desc.clone());
        msg.set_field_by_name("user_id", Value::I64(user_id));
        msg.set_field_by_name("name", Value::String(name.into()));
        msg.encode_to_vec()
    }

    fn make_request(body: Vec<u8>) -> CacheableHttpRequest<Full<Bytes>> {
        let request = http::Request::builder()
            .method("POST")
            .uri("/test.Service/GetUser")
            .body(hitbox_http::BufferedBody::Passthrough(Full::new(
                Bytes::from(body),
            )))
            .unwrap();
        CacheableHttpRequest::from_request(request)
    }

    #[tokio::test]
    async fn extract_int64_field() {
        let (_descriptors, msg_desc) = test_setup();
        let body = encode_request(&msg_desc, 42, "alice");
        let request = make_request(body);

        let extractor = ProtoFieldExtractor::new(msg_desc, "user_id", identity_bytes);
        let key_parts = extractor.get(request).await;
        let (_, cache_key) = key_parts.into_cache_key();

        let parts: Vec<_> = cache_key.parts().collect();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].key(), "user_id");
        assert_eq!(parts[0].value(), Some("42"));
    }

    #[tokio::test]
    async fn extract_string_field() {
        let (_descriptors, msg_desc) = test_setup();
        let body = encode_request(&msg_desc, 42, "alice");
        let request = make_request(body);

        let extractor = ProtoFieldExtractor::new(msg_desc, "name", identity_bytes);
        let key_parts = extractor.get(request).await;
        let (_, cache_key) = key_parts.into_cache_key();

        let parts: Vec<_> = cache_key.parts().collect();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].key(), "name");
        assert_eq!(parts[0].value(), Some("alice"));
    }

    #[tokio::test]
    async fn extract_nonexistent_field() {
        let (_descriptors, msg_desc) = test_setup();
        let body = encode_request(&msg_desc, 42, "alice");
        let request = make_request(body);

        let extractor = ProtoFieldExtractor::new(msg_desc, "nonexistent", identity_bytes);
        let key_parts = extractor.get(request).await;
        let (_, cache_key) = key_parts.into_cache_key();

        let parts: Vec<_> = cache_key.parts().collect();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].key(), "nonexistent");
        assert_eq!(parts[0].value(), None);
    }
}
