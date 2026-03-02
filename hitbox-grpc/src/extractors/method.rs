//! gRPC method name extractor.
//!
//! Extracts the gRPC method name from the request URI path as a cache key part.

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};
use hitbox_http::CacheableHttpRequest;
use hitbox_http::extractors::NeutralExtractor;

use crate::path::GrpcPath;

/// Extracts the gRPC method name as a cache key part.
///
/// Parses the URI path as `/{service}/{method}` and adds
/// `KeyPart::new("grpc_method", Some(method_name))`.
///
/// Typically chained after [`GrpcServiceExtractor`](super::service::GrpcServiceExtractor):
///
/// ```ignore
/// use hitbox_grpc::extractors::service::GrpcServiceExtractor;
///
/// let extractor = GrpcServiceExtractor::new().method();
/// ```
#[derive(Debug)]
pub struct GrpcMethodExtractor<E> {
    inner: E,
}

impl<S> GrpcMethodExtractor<NeutralExtractor<S>> {
    /// Creates a new gRPC method extractor that starts an extractor chain.
    pub fn new() -> Self {
        GrpcMethodExtractor {
            inner: NeutralExtractor::new(),
        }
    }
}

impl<S> Default for GrpcMethodExtractor<NeutralExtractor<S>> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> GrpcMethodExtractor<E> {
    /// Creates a method extractor chained after an existing extractor.
    pub(crate) fn after(inner: E) -> Self {
        GrpcMethodExtractor { inner }
    }

    /// Chains a gRPC proto field extractor using the gRPC bytes extractor.
    pub fn proto_field(
        self,
        message_descriptor: prost_reflect::MessageDescriptor,
        field_path: impl Into<String>,
    ) -> hitbox_proto::extractors::field::ProtoFieldExtractor<Self> {
        hitbox_proto::extractors::field::ProtoFieldExtractor::after(
            self,
            message_descriptor,
            field_path,
            crate::frame::grpc_bytes_extractor,
        )
    }

    /// Chains a proto hash extractor using the gRPC bytes extractor.
    pub fn proto_hash(self) -> hitbox_proto::extractors::hash::ProtoHash<Self> {
        hitbox_proto::extractors::hash::ProtoHash::after(self, crate::frame::grpc_bytes_extractor)
    }
}

/// Extension trait for adding gRPC method extraction to an extractor chain.
pub trait GrpcMethodExtract: Sized {
    /// Adds gRPC method name extraction to this chain.
    fn grpc_method(self) -> GrpcMethodExtractor<Self>;
}

impl<E> GrpcMethodExtract for E
where
    E: Extractor,
{
    fn grpc_method(self) -> GrpcMethodExtractor<Self> {
        GrpcMethodExtractor { inner: self }
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for GrpcMethodExtractor<E>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let method_name =
            GrpcPath::parse(subject.parts().uri.path()).map(|p| p.method().to_string());

        let mut parts = self.inner.get(subject).await;
        parts.push(KeyPart::new("grpc_method", method_name));
        parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::service::GrpcServiceExtractor;
    use bytes::Bytes;
    use http_body_util::Full;

    fn make_request(uri: &str) -> CacheableHttpRequest<Full<Bytes>> {
        let request = http::Request::builder()
            .method("POST")
            .uri(uri)
            .body(hitbox_http::BufferedBody::Passthrough(Full::new(
                Bytes::new(),
            )))
            .unwrap();
        CacheableHttpRequest::from_request(request)
    }

    #[tokio::test]
    async fn extracts_method_name() {
        let extractor = GrpcMethodExtractor::new();
        let request = make_request("/my.pkg.Svc/GetUser");
        let key_parts = extractor.get(request).await;
        let (_, cache_key) = key_parts.into_cache_key();

        let parts: Vec<_> = cache_key.parts().collect();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].key(), "grpc_method");
        assert_eq!(parts[0].value(), Some("GetUser"));
    }

    #[tokio::test]
    async fn service_then_method_chain() {
        let extractor = GrpcServiceExtractor::new().method();
        let request = make_request("/my.pkg.Svc/GetUser");
        let key_parts = extractor.get(request).await;
        let (_, cache_key) = key_parts.into_cache_key();

        let parts: Vec<_> = cache_key.parts().collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].key(), "grpc_service");
        assert_eq!(parts[0].value(), Some("my.pkg.Svc"));
        assert_eq!(parts[1].key(), "grpc_method");
        assert_eq!(parts[1].value(), Some("GetUser"));
    }
}
