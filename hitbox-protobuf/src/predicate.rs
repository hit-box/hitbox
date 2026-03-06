use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};
use hyper::body::Body as HttpBody;
use prost_reflect::MessageDescriptor;
use tracing::debug;

use hitbox_http::{BufferedBody, CacheableSubject};

use crate::decode::{check_operation, decode_message};
use crate::{FieldSpec, FrameDecoder, NoFraming};

/// A predicate that decodes a protobuf body and checks multiple field conditions.
///
/// Decodes the body once (using the provided `FrameDecoder` and `MessageDescriptor`),
/// then evaluates all field specifications against the decoded message. The subject
/// is cacheable only if all field checks pass (AND semantics).
///
/// # Type Parameters
///
/// * `P` - The inner predicate to chain with.
/// * `F` - The frame decoder (default: `NoFraming` for raw protobuf bodies).
///
/// # Examples
///
/// ```ignore
/// use hitbox_protobuf::{ProtoFieldsPredicate, FieldsBuilder, Operation, ProtoValue};
///
/// let fields = FieldsBuilder::new()
///     .field("user_id", Operation::Exists)
///     .field("role", Operation::Eq(ProtoValue::String("admin".into())))
///     .build();
///
/// let predicate = request::predicate::<Body>()
///     .proto_fields(descriptor, fields);
/// ```
#[derive(Debug)]
pub struct ProtoFields<P, F = NoFraming> {
    inner: P,
    descriptor: MessageDescriptor,
    fields: Vec<FieldSpec>,
    frame_decoder: F,
}

/// Extension trait for adding protobuf field checking to a predicate chain.
///
/// This trait is automatically implemented for all [`Predicate`] types.
pub trait ProtoFieldsPredicate: Sized {
    /// Add protobuf field checks with the default `NoFraming` decoder.
    ///
    /// Use this for raw protobuf bodies (e.g., Twirp).
    fn proto_fields(
        self,
        descriptor: MessageDescriptor,
        fields: Vec<FieldSpec>,
    ) -> ProtoFields<Self>;

    /// Add protobuf field checks with a custom frame decoder.
    ///
    /// Use this when the body has protocol-specific framing (e.g., gRPC 5-byte prefix).
    fn proto_fields_with_decoder<F: FrameDecoder>(
        self,
        descriptor: MessageDescriptor,
        fields: Vec<FieldSpec>,
        decoder: F,
    ) -> ProtoFields<Self, F>;
}

impl<P> ProtoFieldsPredicate for P
where
    P: Predicate,
{
    fn proto_fields(
        self,
        descriptor: MessageDescriptor,
        fields: Vec<FieldSpec>,
    ) -> ProtoFields<Self> {
        ProtoFields {
            inner: self,
            descriptor,
            fields,
            frame_decoder: NoFraming,
        }
    }

    fn proto_fields_with_decoder<F: FrameDecoder>(
        self,
        descriptor: MessageDescriptor,
        fields: Vec<FieldSpec>,
        decoder: F,
    ) -> ProtoFields<Self, F> {
        ProtoFields {
            inner: self,
            descriptor,
            fields,
            frame_decoder: decoder,
        }
    }
}

#[async_trait]
impl<P, F> Predicate for ProtoFields<P, F>
where
    P: Predicate + Send + Sync,
    P::Subject: CacheableSubject + Send,
    <P::Subject as CacheableSubject>::Body: Send + Unpin + 'static,
    <P::Subject as CacheableSubject>::Parts: Send,
    <<P::Subject as CacheableSubject>::Body as HttpBody>::Error: Send,
    <<P::Subject as CacheableSubject>::Body as HttpBody>::Data: Send,
    F: FrameDecoder,
{
    type Subject = P::Subject;

    async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject> {
        let inner_result = self.inner.check(subject).await;

        let (was_cacheable, subject) = match inner_result {
            PredicateResult::Cacheable(s) => (true, s),
            PredicateResult::NonCacheable(s) => (false, s),
        };

        let (parts, body) = subject.into_parts();

        // Collect the full body
        let collected = match body.collect().await {
            Ok(c) => c,
            Err(error_body) => {
                debug!("proto_fields: failed to collect body");
                let subject = P::Subject::from_parts(parts, error_body);
                return PredicateResult::NonCacheable(subject);
            }
        };

        let data = collected.data;
        let trailers = collected.trailers;

        // Frame decode → protobuf decode → check fields
        let fields_match = match self.frame_decoder.decode(&data) {
            Ok(proto_bytes) => match decode_message(&self.descriptor, &proto_bytes) {
                Ok(message) => self
                    .fields
                    .iter()
                    .all(|spec| check_operation(&spec.operation, &message, &spec.name)),
                Err(e) => {
                    debug!(%e, "proto_fields: protobuf decode failed");
                    false
                }
            },
            Err(e) => {
                debug!(%e, "proto_fields: frame decode failed");
                false
            }
        };

        // Reconstruct subject with fully buffered body
        let body = BufferedBody::Complete {
            data: Some(data),
            trailers,
        };
        let subject = P::Subject::from_parts(parts, body);

        if was_cacheable && fields_match {
            PredicateResult::Cacheable(subject)
        } else {
            PredicateResult::NonCacheable(subject)
        }
    }
}
