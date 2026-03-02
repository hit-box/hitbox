//! Compile-time typed protobuf extractor.
//!
//! [`TypedProtoExtractor`] decodes to a concrete `prost::Message` type and applies
//! a user-provided closure to extract cache key parts.

use std::marker::PhantomData;

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};
use hyper::body::Body as HttpBody;
use tracing::warn;

use crate::decode::BytesExtractor;

/// Extracts cache key parts from a concrete protobuf message type.
///
/// # Type Parameters
///
/// * `E` - The inner extractor in the chain
/// * `M` - The concrete protobuf message type
/// * `F` - Closure `Fn(&M) -> Vec<KeyPart>` that extracts key parts
#[derive(Debug)]
pub struct TypedProtoExtractor<E, M, F> {
    extract_fn: F,
    bytes_extractor: BytesExtractor,
    inner: E,
    _marker: PhantomData<M>,
}

impl<S, M, F> TypedProtoExtractor<hitbox_http::extractors::NeutralExtractor<S>, M, F>
where
    M: prost::Message + Default,
    F: Fn(&M) -> Vec<KeyPart>,
{
    /// Creates a new typed extractor that starts an extractor chain.
    pub fn new(extract_fn: F, bytes_extractor: BytesExtractor) -> Self {
        TypedProtoExtractor {
            extract_fn,
            bytes_extractor,
            inner: hitbox_http::extractors::NeutralExtractor::new(),
            _marker: PhantomData,
        }
    }
}

/// Extension trait for adding typed proto extraction to an extractor chain.
pub trait TypedProtoExtract: Sized {
    /// Adds typed protobuf extraction to this extractor chain.
    fn typed_proto<M, F>(
        self,
        extract_fn: F,
        bytes_extractor: BytesExtractor,
    ) -> TypedProtoExtractor<Self, M, F>
    where
        M: prost::Message + Default,
        F: Fn(&M) -> Vec<KeyPart>;
}

impl<E> TypedProtoExtract for E
where
    E: Extractor,
{
    fn typed_proto<M, F>(
        self,
        extract_fn: F,
        bytes_extractor: BytesExtractor,
    ) -> TypedProtoExtractor<Self, M, F>
    where
        M: prost::Message + Default,
        F: Fn(&M) -> Vec<KeyPart>,
    {
        TypedProtoExtractor {
            extract_fn,
            bytes_extractor,
            inner: self,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<ReqBody, E, M, F> Extractor for TypedProtoExtractor<E, M, F>
where
    ReqBody: HttpBody + Send + 'static,
    ReqBody::Error: Send,
    ReqBody::Data: Send,
    E: Extractor<Subject = hitbox_http::CacheableHttpRequest<ReqBody>> + Send + Sync,
    M: prost::Message + Default + Send + Sync,
    F: Fn(&M) -> Vec<KeyPart> + Send + Sync,
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
                return self.inner.get(request).await;
            }
        };

        let body_bytes = collected.data;
        let extracted = match crate::decode::decode_typed::<M>(&body_bytes, self.bytes_extractor) {
            Ok(msg) => (self.extract_fn)(&msg),
            Err(e) => {
                warn!(%e, "TypedProtoExtractor: failed to decode protobuf");
                Vec::new()
            }
        };

        let body = hitbox_http::BufferedBody::Complete {
            data: Some(body_bytes),
            trailers: collected.trailers,
        };
        let request =
            hitbox_http::CacheableHttpRequest::from_request(http::Request::from_parts(parts, body));

        let mut key_parts = self.inner.get(request).await;
        for part in extracted {
            key_parts.push(part);
        }
        key_parts
    }
}
