//! Runtime protobuf field predicate using `prost-reflect`.
//!
//! [`ProtoField`] inspects protobuf message fields at runtime using a
//! [`MessageDescriptor`](prost_reflect::MessageDescriptor) to make cache decisions.
//!
//! # Examples
//!
//! ```ignore
//! use hitbox_proto::predicates::field::{ProtoField, FieldOp};
//! use hitbox_proto::descriptor::ProtoDescriptors;
//! use hitbox_proto::decode::identity_bytes;
//! use prost_reflect::Value;
//!
//! let descriptors = ProtoDescriptors::from_bytes(descriptor_bytes).unwrap();
//! let msg_desc = descriptors.get_message("my.pkg.GetUserRequest").unwrap();
//!
//! // Only cache requests where user_id == 42
//! let predicate = ProtoField::new(
//!     msg_desc,
//!     "user_id",
//!     FieldOp::eq(Value::I64(42)),
//!     identity_bytes,
//! );
//! ```

use async_trait::async_trait;
use hitbox::Neutral;
use hitbox::predicate::{Predicate, PredicateResult};
use hyper::body::Body as HttpBody;
use prost_reflect::{MessageDescriptor, Value};
use tracing::warn;

use crate::CacheableSubject;
use crate::decode::BytesExtractor;

/// Comparison operations for protobuf field values.
#[derive(Debug, Clone)]
pub enum FieldOp {
    /// Field value must equal this value.
    Eq(Value),
    /// Field value must not equal this value.
    NotEq(Value),
    /// Field value must be one of these values.
    In(Vec<Value>),
    /// Field must be present (non-default for proto3).
    Exists,
}

impl FieldOp {
    /// Creates an `Eq` operation with the given [`Value`].
    pub fn eq(value: Value) -> Self {
        FieldOp::Eq(value)
    }

    /// Creates a `NotEq` operation with the given [`Value`].
    pub fn not_eq(value: Value) -> Self {
        FieldOp::NotEq(value)
    }

    /// Creates an `In` operation with the given [`Value`]s.
    pub fn is_in(values: Vec<Value>) -> Self {
        FieldOp::In(values)
    }

    fn check(&self, field_value: &Value) -> bool {
        match self {
            FieldOp::Eq(expected) => field_value == expected,
            FieldOp::NotEq(expected) => field_value != expected,
            FieldOp::In(allowed) => allowed.iter().any(|v| v == field_value),
            FieldOp::Exists => !is_default_value(field_value),
        }
    }
}

/// Check if a proto3 value is its default (zero/empty).
fn is_default_value(value: &Value) -> bool {
    match value {
        Value::Bool(b) => !b,
        Value::I32(n) => *n == 0,
        Value::I64(n) => *n == 0,
        Value::U32(n) => *n == 0,
        Value::U64(n) => *n == 0,
        Value::F32(n) => *n == 0.0,
        Value::F64(n) => *n == 0.0,
        Value::String(s) => s.is_empty(),
        Value::Bytes(b) => b.is_empty(),
        Value::EnumNumber(n) => *n == 0,
        Value::List(l) => l.is_empty(),
        Value::Map(m) => m.is_empty(),
        Value::Message(_) => false, // Messages are always "present" if set
    }
}

/// A predicate that inspects a protobuf message field at runtime.
///
/// Uses [`prost-reflect`](prost_reflect) to decode the request body as a
/// [`DynamicMessage`](prost_reflect::DynamicMessage) and check a field value.
///
/// The body is consumed during inspection and returned as
/// [`BufferedBody::Complete`](hitbox_http::BufferedBody) afterward.
///
/// # Type Parameters
///
/// * `P` - The inner predicate in the chain.
#[derive(Debug)]
pub struct ProtoField<P> {
    message_descriptor: MessageDescriptor,
    field_path: String,
    operation: FieldOp,
    bytes_extractor: BytesExtractor,
    inner: P,
}

impl<S> ProtoField<Neutral<S>> {
    /// Creates a new `ProtoField` predicate that starts a predicate chain.
    ///
    /// # Arguments
    ///
    /// * `message_descriptor` - The protobuf message type descriptor
    /// * `field_path` - The field name to inspect (e.g., `"user_id"`)
    /// * `operation` - The comparison to perform
    /// * `bytes_extractor` - Pre-processing function for raw bytes (e.g., gRPC frame stripping)
    pub fn new(
        message_descriptor: MessageDescriptor,
        field_path: impl Into<String>,
        operation: FieldOp,
        bytes_extractor: BytesExtractor,
    ) -> Self {
        ProtoField {
            message_descriptor,
            field_path: field_path.into(),
            operation,
            bytes_extractor,
            inner: Neutral::new(),
        }
    }
}

/// Extension trait for adding protobuf field matching to a predicate chain.
pub trait ProtoFieldPredicate: Sized {
    /// Adds a protobuf field check to this predicate chain.
    fn proto_field(
        self,
        message_descriptor: MessageDescriptor,
        field_path: impl Into<String>,
        operation: FieldOp,
        bytes_extractor: BytesExtractor,
    ) -> ProtoField<Self>;
}

impl<P> ProtoFieldPredicate for P
where
    P: Predicate,
{
    fn proto_field(
        self,
        message_descriptor: MessageDescriptor,
        field_path: impl Into<String>,
        operation: FieldOp,
        bytes_extractor: BytesExtractor,
    ) -> ProtoField<Self> {
        ProtoField {
            message_descriptor,
            field_path: field_path.into(),
            operation,
            bytes_extractor,
            inner: self,
        }
    }
}

#[async_trait]
impl<P> Predicate for ProtoField<P>
where
    P: Predicate + Send + Sync,
    P::Subject: CacheableSubject + Send,
    <P::Subject as CacheableSubject>::Body: Send + Unpin + 'static,
    <P::Subject as CacheableSubject>::Parts: Send,
    <<P::Subject as CacheableSubject>::Body as HttpBody>::Error: Send,
    <<P::Subject as CacheableSubject>::Body as HttpBody>::Data: Send,
{
    type Subject = P::Subject;

    async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject> {
        let inner_result = self.inner.check(subject).await;

        let (was_cacheable, subject) = match inner_result {
            PredicateResult::Cacheable(s) => (true, s),
            PredicateResult::NonCacheable(s) => (false, s),
        };

        let (parts, body) = subject.into_parts();

        // Collect the body bytes
        let collected = match body.collect().await {
            Ok(collected) => collected,
            Err(error_body) => {
                warn!("ProtoField: failed to collect body");
                return PredicateResult::NonCacheable(P::Subject::from_parts(parts, error_body));
            }
        };

        let body_bytes = collected.data;
        // Decode the protobuf message
        let field_matches = match crate::decode::decode_dynamic(
            &self.message_descriptor,
            &body_bytes,
            self.bytes_extractor,
        ) {
            Ok(msg) => match msg.get_field_by_name(&self.field_path) {
                Some(value) => self.operation.check(value.as_ref()),
                None => {
                    warn!(field = %self.field_path, "ProtoField: field not found in message");
                    false
                }
            },
            Err(e) => {
                warn!(%e, "ProtoField: failed to decode protobuf message");
                false
            }
        };

        // Reconstruct with complete body
        let body = hitbox_http::BufferedBody::Complete {
            data: Some(body_bytes),
            trailers: collected.trailers,
        };
        let subject = P::Subject::from_parts(parts, body);

        if was_cacheable && field_matches {
            PredicateResult::Cacheable(subject)
        } else {
            PredicateResult::NonCacheable(subject)
        }
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
    use prost_reflect::DynamicMessage;

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

    fn make_cacheable_request(body: Vec<u8>) -> CacheableHttpRequest<Full<Bytes>> {
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
    async fn proto_field_eq_cacheable() {
        let (_descriptors, msg_desc) = test_setup();
        let body = encode_request(&msg_desc, 42, "alice");
        let request = make_cacheable_request(body);

        let predicate = ProtoField::new(
            msg_desc,
            "user_id",
            FieldOp::Eq(Value::I64(42)),
            identity_bytes,
        );

        let result = predicate.check(request).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn proto_field_eq_non_cacheable() {
        let (_descriptors, msg_desc) = test_setup();
        let body = encode_request(&msg_desc, 99, "bob");
        let request = make_cacheable_request(body);

        let predicate = ProtoField::new(
            msg_desc,
            "user_id",
            FieldOp::Eq(Value::I64(42)),
            identity_bytes,
        );

        let result = predicate.check(request).await;
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn proto_field_not_eq() {
        let (_descriptors, msg_desc) = test_setup();
        let body = encode_request(&msg_desc, 42, "alice");
        let request = make_cacheable_request(body);

        let predicate = ProtoField::new(
            msg_desc,
            "user_id",
            FieldOp::NotEq(Value::I64(99)),
            identity_bytes,
        );

        let result = predicate.check(request).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn proto_field_in() {
        let (_descriptors, msg_desc) = test_setup();
        let body = encode_request(&msg_desc, 42, "alice");
        let request = make_cacheable_request(body);

        let predicate = ProtoField::new(
            msg_desc,
            "user_id",
            FieldOp::In(vec![Value::I64(1), Value::I64(42), Value::I64(100)]),
            identity_bytes,
        );

        let result = predicate.check(request).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn proto_field_exists_present() {
        let (_descriptors, msg_desc) = test_setup();
        let body = encode_request(&msg_desc, 42, "alice");
        let request = make_cacheable_request(body);

        let predicate = ProtoField::new(msg_desc, "name", FieldOp::Exists, identity_bytes);

        let result = predicate.check(request).await;
        assert!(matches!(result, PredicateResult::Cacheable(_)));
    }

    #[tokio::test]
    async fn proto_field_exists_default() {
        let (_descriptors, msg_desc) = test_setup();
        // Empty name = default value for string
        let body = encode_request(&msg_desc, 42, "");
        let request = make_cacheable_request(body);

        let predicate = ProtoField::new(msg_desc, "name", FieldOp::Exists, identity_bytes);

        let result = predicate.check(request).await;
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn proto_field_nonexistent_field() {
        let (_descriptors, msg_desc) = test_setup();
        let body = encode_request(&msg_desc, 42, "alice");
        let request = make_cacheable_request(body);

        let predicate = ProtoField::new(msg_desc, "nonexistent", FieldOp::Exists, identity_bytes);

        let result = predicate.check(request).await;
        assert!(matches!(result, PredicateResult::NonCacheable(_)));
    }

    #[tokio::test]
    async fn proto_field_preserves_body() {
        let (_descriptors, msg_desc) = test_setup();
        let original_body = encode_request(&msg_desc, 42, "alice");
        let request = make_cacheable_request(original_body.clone());

        let predicate = ProtoField::new(
            msg_desc,
            "user_id",
            FieldOp::Eq(Value::I64(42)),
            identity_bytes,
        );

        let result = predicate.check(request).await;
        match result {
            PredicateResult::Cacheable(req) => {
                let (_, body) = req.into_parts();
                let collected = body.collect().await.unwrap();
                assert_eq!(collected.data.as_ref(), &original_body);
            }
            _ => panic!("expected Cacheable"),
        }
    }
}
