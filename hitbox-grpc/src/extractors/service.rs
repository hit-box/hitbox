//! gRPC service name extractor.
//!
//! Extracts the gRPC service name from the request URI path as a cache key part.

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};
use hitbox_http::CacheableHttpRequest;
use hitbox_http::extractors::NeutralExtractor;

use crate::path::GrpcPath;

/// Extracts the gRPC service name as a cache key part.
///
/// Parses the URI path as `/{service}/{method}` and adds
/// `KeyPart::new("grpc_service", Some(service_name))`.
///
/// # Examples
///
/// ```ignore
/// use hitbox_grpc::extractors::service::GrpcServiceExtractor;
///
/// let extractor = GrpcServiceExtractor::new();
/// ```
#[derive(Debug)]
pub struct GrpcServiceExtractor<E> {
    inner: E,
}

impl<S> GrpcServiceExtractor<NeutralExtractor<S>> {
    /// Creates a new gRPC service extractor that starts an extractor chain.
    pub fn new() -> Self {
        GrpcServiceExtractor {
            inner: NeutralExtractor::new(),
        }
    }
}

impl<S> Default for GrpcServiceExtractor<NeutralExtractor<S>> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E> GrpcServiceExtractor<E> {
    /// Chains a [`GrpcMethodExtractor`](super::method::GrpcMethodExtractor) after this.
    ///
    /// Enables: `GrpcServiceExtractor::new().method()`
    pub fn method(self) -> super::method::GrpcMethodExtractor<Self> {
        super::method::GrpcMethodExtractor::after(self)
    }
}

/// Extension trait for adding gRPC service extraction to an extractor chain.
pub trait GrpcServiceExtract: Sized {
    /// Adds gRPC service name extraction to this chain.
    fn grpc_service(self) -> GrpcServiceExtractor<Self>;
}

impl<E> GrpcServiceExtract for E
where
    E: Extractor,
{
    fn grpc_service(self) -> GrpcServiceExtractor<Self> {
        GrpcServiceExtractor { inner: self }
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for GrpcServiceExtractor<E>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let service_name =
            GrpcPath::parse(subject.parts().uri.path()).map(|p| p.service().to_string());

        let mut parts = self.inner.get(subject).await;
        parts.push(KeyPart::new("grpc_service", service_name));
        parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    async fn extracts_service_name() {
        let extractor = GrpcServiceExtractor::new();
        let request = make_request("/my.pkg.UserService/GetUser");
        let key_parts = extractor.get(request).await;
        let (_, cache_key) = key_parts.into_cache_key();

        let parts: Vec<_> = cache_key.parts().collect();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].key(), "grpc_service");
        assert_eq!(parts[0].value(), Some("my.pkg.UserService"));
    }

    #[tokio::test]
    async fn invalid_path_produces_none_value() {
        let extractor = GrpcServiceExtractor::new();
        let request = make_request("/not-grpc");
        let key_parts = extractor.get(request).await;
        let (_, cache_key) = key_parts.into_cache_key();

        let parts: Vec<_> = cache_key.parts().collect();
        assert_eq!(parts[0].key(), "grpc_service");
        assert_eq!(parts[0].value(), None);
    }
}
