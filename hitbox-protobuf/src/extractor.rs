use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};
use hyper::body::Body as HttpBody;
use prost_reflect::MessageDescriptor;
use tracing::debug;

use hitbox_http::{BufferedBody, CacheableHttpRequest};

use crate::decode::{
    decode_message, extract_field, resolve_field_descriptor, value_to_key_string_ext,
};
use crate::{FrameDecoder, NoFraming};

/// Extracts cache key parts from protobuf request body fields.
///
/// Decodes the body once (using the provided `FrameDecoder` and `MessageDescriptor`),
/// then extracts specified field values as [`KeyPart`] entries for cache key generation.
///
/// # Type Parameters
///
/// * `E` - The inner extractor to chain with.
/// * `F` - The frame decoder (default: `NoFraming` for raw protobuf bodies).
///
/// # Examples
///
/// ```ignore
/// use hitbox_protobuf::ProtoFieldsExtract;
///
/// let extractor = extractors::extractor::<Body>()
///     .proto_fields(descriptor, vec!["user_id".into(), "role".into()]);
/// ```
#[derive(Debug)]
pub struct ProtoFieldsExtractor<E, F = NoFraming> {
    inner: E,
    descriptor: MessageDescriptor,
    fields: Vec<String>,
    frame_decoder: F,
}

/// Extension trait for adding protobuf field extraction to an extractor chain.
///
/// This trait is automatically implemented for all [`Extractor`] types.
pub trait ProtoFieldsExtract: Sized {
    /// Add protobuf field extraction with the default `NoFraming` decoder.
    fn proto_fields(
        self,
        descriptor: MessageDescriptor,
        fields: Vec<String>,
    ) -> ProtoFieldsExtractor<Self>;

    /// Add protobuf field extraction with a custom frame decoder.
    fn proto_fields_with_decoder<F: FrameDecoder>(
        self,
        descriptor: MessageDescriptor,
        fields: Vec<String>,
        decoder: F,
    ) -> ProtoFieldsExtractor<Self, F>;
}

impl<E> ProtoFieldsExtract for E
where
    E: Extractor,
{
    fn proto_fields(
        self,
        descriptor: MessageDescriptor,
        fields: Vec<String>,
    ) -> ProtoFieldsExtractor<Self> {
        ProtoFieldsExtractor {
            inner: self,
            descriptor,
            fields,
            frame_decoder: NoFraming,
        }
    }

    fn proto_fields_with_decoder<F: FrameDecoder>(
        self,
        descriptor: MessageDescriptor,
        fields: Vec<String>,
        decoder: F,
    ) -> ProtoFieldsExtractor<Self, F> {
        ProtoFieldsExtractor {
            inner: self,
            descriptor,
            fields,
            frame_decoder: decoder,
        }
    }
}

#[async_trait]
impl<ReqBody, E, F> Extractor for ProtoFieldsExtractor<E, F>
where
    ReqBody: HttpBody + Send + 'static,
    ReqBody::Error: Send,
    ReqBody::Data: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
    F: FrameDecoder,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let (parts, body) = subject.into_parts();

        // Collect the full body
        let collected = match body.collect().await {
            Ok(c) => c,
            Err(error_body) => {
                debug!("proto_fields extractor: failed to collect body");
                let request = CacheableHttpRequest::from_request(http::Request::from_parts(
                    parts, error_body,
                ));
                return self.inner.get(request).await;
            }
        };

        let data = collected.data;
        let trailers = collected.trailers;

        // Frame decode → protobuf decode → extract fields
        let extracted_parts = match self.frame_decoder.decode(&data) {
            Ok(proto_bytes) => match decode_message(&self.descriptor, &proto_bytes) {
                Ok(message) => self
                    .fields
                    .iter()
                    .map(|field_name| {
                        let value = extract_field(&message, field_name);
                        let field_desc = resolve_field_descriptor(&message, field_name);
                        let key_value = value
                            .as_ref()
                            .and_then(|v| value_to_key_string_ext(v, field_desc.as_ref()));
                        KeyPart::new(field_name.clone(), key_value)
                    })
                    .collect::<Vec<_>>(),
                Err(e) => {
                    debug!(%e, "proto_fields extractor: protobuf decode failed");
                    Vec::new()
                }
            },
            Err(e) => {
                debug!(%e, "proto_fields extractor: frame decode failed");
                Vec::new()
            }
        };

        // Reconstruct request with fully buffered body
        let body = BufferedBody::Complete {
            data: Some(data),
            trailers,
        };
        let request = CacheableHttpRequest::from_request(http::Request::from_parts(parts, body));

        let mut key_parts = self.inner.get(request).await;
        for part in extracted_parts {
            key_parts.push(part);
        }
        key_parts
    }
}
