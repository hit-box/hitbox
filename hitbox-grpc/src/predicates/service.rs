//! gRPC service name predicate.
//!
//! [`GrpcService`] matches requests by their gRPC service name parsed from the URI path.

use async_trait::async_trait;
use hitbox::Neutral;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::CacheableHttpRequest;

use crate::path::GrpcPath;

/// A predicate that matches requests by gRPC service name.
///
/// Parses the URI path as `/{service}/{method}` and compares the service
/// name against the configured value.
///
/// # Examples
///
/// ```ignore
/// use hitbox_grpc::predicates::service::GrpcService;
///
/// // Only cache requests to the UserService
/// let predicate = GrpcService::new("my.package.UserService");
/// ```
#[derive(Debug)]
pub struct GrpcService<P> {
    service_name: String,
    inner: P,
}

impl<S> GrpcService<Neutral<S>> {
    /// Creates a predicate matching the specified gRPC service name.
    pub fn new(service_name: impl Into<String>) -> Self {
        GrpcService {
            service_name: service_name.into(),
            inner: Neutral::new(),
        }
    }
}

impl<P> GrpcService<P> {
    /// Chains a [`GrpcMethod`](super::method::GrpcMethod) predicate after this service predicate.
    ///
    /// This enables the fluent pattern: `GrpcService::new("svc").method("GetUser")`
    pub fn method(self, name: impl Into<String>) -> super::method::GrpcMethod<Self> {
        super::method::GrpcMethod::after(self, super::method::MethodMatch::Exact(name.into()))
    }

    /// Chains a [`GrpcMethod`](super::method::GrpcMethod) predicate matching any of the given methods.
    pub fn methods(self, names: Vec<String>) -> super::method::GrpcMethod<Self> {
        super::method::GrpcMethod::after(self, super::method::MethodMatch::In(names))
    }
}

/// Extension trait for adding gRPC service matching to a predicate chain.
pub trait GrpcServicePredicate: Sized {
    /// Adds gRPC service name matching to this predicate chain.
    fn grpc_service(self, service_name: impl Into<String>) -> GrpcService<Self>;
}

impl<P> GrpcServicePredicate for P
where
    P: Predicate,
{
    fn grpc_service(self, service_name: impl Into<String>) -> GrpcService<Self> {
        GrpcService {
            service_name: service_name.into(),
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for GrpcService<P>
where
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
{
    type Subject = P::Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(request).await {
            PredicateResult::Cacheable(request) => {
                let matches = GrpcPath::parse(request.parts().uri.path())
                    .is_some_and(|path| path.service() == self.service_name);
                if matches {
                    PredicateResult::Cacheable(request)
                } else {
                    PredicateResult::NonCacheable(request)
                }
            }
            non_cacheable => non_cacheable,
        }
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
    async fn matches_service() {
        let predicate = GrpcService::new("my.pkg.UserService");
        let request = make_request("/my.pkg.UserService/GetUser");
        assert!(matches!(
            predicate.check(request).await,
            PredicateResult::Cacheable(_)
        ));
    }

    #[tokio::test]
    async fn rejects_different_service() {
        let predicate = GrpcService::new("my.pkg.UserService");
        let request = make_request("/my.pkg.OrderService/GetOrder");
        assert!(matches!(
            predicate.check(request).await,
            PredicateResult::NonCacheable(_)
        ));
    }

    #[tokio::test]
    async fn rejects_invalid_path() {
        let predicate = GrpcService::new("my.pkg.UserService");
        let request = make_request("/not-grpc");
        assert!(matches!(
            predicate.check(request).await,
            PredicateResult::NonCacheable(_)
        ));
    }
}
