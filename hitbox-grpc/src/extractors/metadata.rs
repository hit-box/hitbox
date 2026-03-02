//! gRPC metadata (headers) extractor.
//!
//! Extracts gRPC metadata from request headers as cache key parts.
//! Binary metadata headers (suffix `-bin`) are base64-decoded and hex-encoded
//! for readable cache keys.

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};
use hitbox_http::CacheableHttpRequest;
use hitbox_http::extractors::NeutralExtractor;

/// Extracts specific gRPC metadata headers as cache key parts.
///
/// For ASCII headers, the value is used directly.
/// For binary headers (ending in `-bin`), the value is base64-decoded
/// and then hex-encoded for a readable cache key.
///
/// # Examples
///
/// ```ignore
/// use hitbox_grpc::extractors::metadata::GrpcMetadata;
///
/// // Extract authorization and tenant-id metadata
/// let extractor = GrpcMetadata::new(vec!["authorization".into(), "x-tenant-id".into()]);
/// ```
#[derive(Debug)]
pub struct GrpcMetadata<E> {
    header_names: Vec<String>,
    inner: E,
}

impl<S> GrpcMetadata<NeutralExtractor<S>> {
    /// Creates a new metadata extractor for the specified header names.
    pub fn new(header_names: Vec<String>) -> Self {
        GrpcMetadata {
            header_names,
            inner: NeutralExtractor::new(),
        }
    }
}

/// Extension trait for adding gRPC metadata extraction to an extractor chain.
pub trait GrpcMetadataExtract: Sized {
    /// Adds gRPC metadata extraction for the specified headers.
    fn grpc_metadata(self, header_names: Vec<String>) -> GrpcMetadata<Self>;
}

impl<E> GrpcMetadataExtract for E
where
    E: Extractor,
{
    fn grpc_metadata(self, header_names: Vec<String>) -> GrpcMetadata<Self> {
        GrpcMetadata {
            header_names,
            inner: self,
        }
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for GrpcMetadata<E>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let mut extracted = Vec::new();

        for name in &self.header_names {
            let value = subject.parts().headers.get(name).map(|v| {
                if name.ends_with("-bin") {
                    // Binary metadata: base64-decode, then hex-encode for readability
                    match base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        v.as_bytes(),
                    ) {
                        Ok(decoded) => hex::encode(decoded),
                        Err(_) => {
                            // Fall back to raw value if base64 decode fails
                            String::from_utf8_lossy(v.as_bytes()).to_string()
                        }
                    }
                } else {
                    String::from_utf8_lossy(v.as_bytes()).to_string()
                }
            });
            extracted.push(KeyPart::new(format!("grpc_meta.{name}"), value));
        }

        let mut parts = self.inner.get(subject).await;
        for part in extracted {
            parts.push(part);
        }
        parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http_body_util::Full;

    fn make_request_with_headers(headers: Vec<(&str, &str)>) -> CacheableHttpRequest<Full<Bytes>> {
        let mut builder = http::Request::builder().method("POST").uri("/svc/Method");
        for (name, value) in headers {
            builder = builder.header(name, value);
        }
        let request = builder
            .body(hitbox_http::BufferedBody::Passthrough(Full::new(
                Bytes::new(),
            )))
            .unwrap();
        CacheableHttpRequest::from_request(request)
    }

    #[tokio::test]
    async fn extracts_ascii_metadata() {
        let extractor = GrpcMetadata::new(vec!["x-tenant-id".into()]);
        let request = make_request_with_headers(vec![("x-tenant-id", "tenant-42")]);
        let key_parts = extractor.get(request).await;
        let (_, cache_key) = key_parts.into_cache_key();

        let parts: Vec<_> = cache_key.parts().collect();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].key(), "grpc_meta.x-tenant-id");
        assert_eq!(parts[0].value(), Some("tenant-42"));
    }

    #[tokio::test]
    async fn missing_header_produces_none() {
        let extractor = GrpcMetadata::new(vec!["x-tenant-id".into()]);
        let request = make_request_with_headers(vec![]);
        let key_parts = extractor.get(request).await;
        let (_, cache_key) = key_parts.into_cache_key();

        let parts: Vec<_> = cache_key.parts().collect();
        assert_eq!(parts[0].key(), "grpc_meta.x-tenant-id");
        assert_eq!(parts[0].value(), None);
    }

    #[tokio::test]
    async fn extracts_binary_metadata() {
        use base64::Engine;
        let binary_value = base64::engine::general_purpose::STANDARD.encode(b"\x01\x02\x03");

        let extractor = GrpcMetadata::new(vec!["data-bin".into()]);
        let request = make_request_with_headers(vec![("data-bin", &binary_value)]);
        let key_parts = extractor.get(request).await;
        let (_, cache_key) = key_parts.into_cache_key();

        let parts: Vec<_> = cache_key.parts().collect();
        assert_eq!(parts[0].key(), "grpc_meta.data-bin");
        assert_eq!(parts[0].value(), Some("010203")); // hex-encoded
    }

    #[tokio::test]
    async fn multiple_headers() {
        let extractor = GrpcMetadata::new(vec!["x-tenant-id".into(), "x-user-id".into()]);
        let request = make_request_with_headers(vec![("x-tenant-id", "t1"), ("x-user-id", "u42")]);
        let key_parts = extractor.get(request).await;
        let (_, cache_key) = key_parts.into_cache_key();

        let parts: Vec<_> = cache_key.parts().collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].key(), "grpc_meta.x-tenant-id");
        assert_eq!(parts[0].value(), Some("t1"));
        assert_eq!(parts[1].key(), "grpc_meta.x-user-id");
        assert_eq!(parts[1].value(), Some("u42"));
    }
}
