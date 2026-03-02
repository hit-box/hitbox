//! Protobuf body hash extractor.
//!
//! [`ProtoHash`] hashes the raw protobuf bytes (after applying a [`BytesExtractor`](crate::decode::BytesExtractor))
//! with SHA-256 and uses the truncated hash as a cache key part.
//!
//! This is useful when you need a cache key component based on the full message
//! content but don't want to inspect individual fields.

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};
use hyper::body::Body as HttpBody;
use sha2::{Digest, Sha256};
use tracing::warn;

use crate::decode::BytesExtractor;

/// Hashes protobuf body bytes for use as a cache key part.
///
/// Applies the `bytes_extractor` to strip framing (e.g., gRPC 5-byte header),
/// then SHA-256 hashes the payload, truncated to 16 hex characters.
///
/// Generates `KeyPart::new("proto_hash", Some(hex_hash))`.
///
/// # Type Parameters
///
/// * `E` - The inner extractor in the chain
#[derive(Debug)]
pub struct ProtoHash<E> {
    bytes_extractor: BytesExtractor,
    inner: E,
}

impl<S> ProtoHash<hitbox_http::extractors::NeutralExtractor<S>> {
    /// Creates a new proto hash extractor that starts an extractor chain.
    pub fn new(bytes_extractor: BytesExtractor) -> Self {
        ProtoHash {
            bytes_extractor,
            inner: hitbox_http::extractors::NeutralExtractor::new(),
        }
    }
}

impl<E> ProtoHash<E> {
    /// Creates a proto hash extractor chained after an existing extractor.
    ///
    /// Unlike the [`ProtoHashExtract`] extension trait, this does not require
    /// `E: Extractor` at construction time, making it suitable for use in
    /// convenience methods on protocol-specific extractors.
    pub fn after(inner: E, bytes_extractor: BytesExtractor) -> Self {
        ProtoHash {
            bytes_extractor,
            inner,
        }
    }
}

/// Extension trait for adding proto hash extraction to an extractor chain.
pub trait ProtoHashExtract: Sized {
    /// Adds protobuf body hash extraction to this extractor chain.
    fn proto_hash(self, bytes_extractor: BytesExtractor) -> ProtoHash<Self>;
}

impl<E> ProtoHashExtract for E
where
    E: Extractor,
{
    fn proto_hash(self, bytes_extractor: BytesExtractor) -> ProtoHash<Self> {
        ProtoHash {
            bytes_extractor,
            inner: self,
        }
    }
}

/// Compute SHA-256 hash truncated to 16 hex chars.
fn hash_bytes(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    let full_hex = hex::encode(digest);
    full_hex[..16].to_string()
}

#[async_trait]
impl<ReqBody, E> Extractor for ProtoHash<E>
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
                key_parts.push(KeyPart::new("proto_hash", None::<String>));
                return key_parts;
            }
        };

        let body_bytes = collected.data;
        let hash_part = match (self.bytes_extractor)(&body_bytes) {
            Some(payload) => {
                let hash = hash_bytes(payload);
                KeyPart::new("proto_hash", Some(hash))
            }
            None => {
                warn!("ProtoHash: bytes extraction failed");
                KeyPart::new("proto_hash", None::<String>)
            }
        };

        let body = hitbox_http::BufferedBody::Complete {
            data: Some(body_bytes),
            trailers: collected.trailers,
        };
        let request =
            hitbox_http::CacheableHttpRequest::from_request(http::Request::from_parts(parts, body));

        let mut key_parts = self.inner.get(request).await;
        key_parts.push(hash_part);
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

    fn encode_test_message() -> Vec<u8> {
        let descriptors = crate::test_util::test_descriptors();
        let msg_desc = descriptors.get_message("test.Ping").unwrap();
        let mut msg = DynamicMessage::new(msg_desc);
        msg.set_field_by_name("msg", Value::String("hello".into()));
        msg.encode_to_vec()
    }

    fn make_request(body: Vec<u8>) -> CacheableHttpRequest<Full<Bytes>> {
        let request = http::Request::builder()
            .method("POST")
            .uri("/test/Ping")
            .body(hitbox_http::BufferedBody::Passthrough(Full::new(
                Bytes::from(body),
            )))
            .unwrap();
        CacheableHttpRequest::from_request(request)
    }

    #[tokio::test]
    async fn hash_produces_consistent_key() {
        let body = encode_test_message();
        let request = make_request(body.clone());

        let extractor = ProtoHash::new(identity_bytes);
        let key_parts = extractor.get(request).await;
        let (_, cache_key) = key_parts.into_cache_key();

        let parts: Vec<_> = cache_key.parts().collect();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].key(), "proto_hash");
        assert!(parts[0].value().is_some());

        // Same input should produce same hash
        let request2 = make_request(body);
        let key_parts2 = extractor.get(request2).await;
        let (_, cache_key2) = key_parts2.into_cache_key();
        let parts2: Vec<_> = cache_key2.parts().collect();
        assert_eq!(parts[0].value(), parts2[0].value());
    }

    #[tokio::test]
    async fn different_bodies_produce_different_hashes() {
        let body1 = encode_test_message();

        // Create a different message with different content
        let descriptors = crate::test_util::test_descriptors();
        let msg_desc = descriptors.get_message("test.Ping").unwrap();
        let mut msg = DynamicMessage::new(msg_desc);
        msg.set_field_by_name("msg", Value::String("world".into()));
        let body2 = msg.encode_to_vec();

        let extractor = ProtoHash::new(identity_bytes);

        let kp1 = extractor.get(make_request(body1)).await;
        let (_, ck1) = kp1.into_cache_key();
        let kp2 = extractor.get(make_request(body2)).await;
        let (_, ck2) = kp2.into_cache_key();

        let p1: Vec<_> = ck1.parts().collect();
        let p2: Vec<_> = ck2.parts().collect();
        assert_ne!(p1[0].value(), p2[0].value());
    }

    #[tokio::test]
    async fn hash_is_16_hex_chars() {
        let body = encode_test_message();
        let request = make_request(body);

        let extractor = ProtoHash::new(identity_bytes);
        let key_parts = extractor.get(request).await;
        let (_, cache_key) = key_parts.into_cache_key();

        let parts: Vec<_> = cache_key.parts().collect();
        let hash = parts[0].value().unwrap();
        assert_eq!(hash.len(), 16);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
