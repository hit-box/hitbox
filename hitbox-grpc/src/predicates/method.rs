//! gRPC method name predicate.
//!
//! [`GrpcMethod`] matches requests by their gRPC method name parsed from the URI path.

use async_trait::async_trait;
use hitbox::Neutral;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::CacheableHttpRequest;

use crate::path::GrpcPath;

/// How to match gRPC method names.
#[derive(Debug, Clone)]
pub enum MethodMatch {
    /// Match a single method name exactly.
    Exact(String),
    /// Match any of the given method names.
    In(Vec<String>),
}

/// A predicate that matches requests by gRPC method name.
///
/// Typically chained after [`GrpcService`](super::service::GrpcService):
///
/// ```ignore
/// use hitbox_grpc::predicates::service::GrpcService;
///
/// let predicate = GrpcService::new("my.pkg.Svc").method("GetUser");
/// ```
#[derive(Debug)]
pub struct GrpcMethod<P> {
    method_match: MethodMatch,
    inner: P,
}

impl<S> GrpcMethod<Neutral<S>> {
    /// Creates a predicate matching the specified gRPC method name.
    pub fn new(name: impl Into<String>) -> Self {
        GrpcMethod {
            method_match: MethodMatch::Exact(name.into()),
            inner: Neutral::new(),
        }
    }

    /// Creates a predicate matching any of the specified gRPC method names.
    pub fn new_in(names: Vec<String>) -> Self {
        GrpcMethod {
            method_match: MethodMatch::In(names),
            inner: Neutral::new(),
        }
    }
}

impl<P> GrpcMethod<P> {
    /// Creates a method predicate chained after an existing predicate.
    pub(crate) fn after(inner: P, method_match: MethodMatch) -> Self {
        GrpcMethod {
            method_match,
            inner,
        }
    }
}

/// Extension trait for adding gRPC method matching to a predicate chain.
pub trait GrpcMethodPredicate: Sized {
    /// Adds gRPC method name matching to this predicate chain.
    fn grpc_method(self, name: impl Into<String>) -> GrpcMethod<Self>;
}

impl<P> GrpcMethodPredicate for P
where
    P: Predicate,
{
    fn grpc_method(self, name: impl Into<String>) -> GrpcMethod<Self> {
        GrpcMethod {
            method_match: MethodMatch::Exact(name.into()),
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for GrpcMethod<P>
where
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
{
    type Subject = P::Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(request).await {
            PredicateResult::Cacheable(request) => {
                let matches = GrpcPath::parse(request.parts().uri.path()).is_some_and(|path| {
                    match &self.method_match {
                        MethodMatch::Exact(name) => path.method() == name,
                        MethodMatch::In(names) => names.iter().any(|n| n == path.method()),
                    }
                });
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
    use crate::predicates::service::GrpcService;
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
    async fn service_then_method() {
        let predicate = GrpcService::new("my.pkg.Svc").method("GetUser");
        let request = make_request("/my.pkg.Svc/GetUser");
        assert!(matches!(
            predicate.check(request).await,
            PredicateResult::Cacheable(_)
        ));
    }

    #[tokio::test]
    async fn service_match_method_mismatch() {
        let predicate = GrpcService::new("my.pkg.Svc").method("GetUser");
        let request = make_request("/my.pkg.Svc/ListUsers");
        assert!(matches!(
            predicate.check(request).await,
            PredicateResult::NonCacheable(_)
        ));
    }

    #[tokio::test]
    async fn service_mismatch_short_circuits() {
        let predicate = GrpcService::new("my.pkg.Svc").method("GetUser");
        let request = make_request("/other.Svc/GetUser");
        assert!(matches!(
            predicate.check(request).await,
            PredicateResult::NonCacheable(_)
        ));
    }

    #[tokio::test]
    async fn method_in_matches() {
        let predicate =
            GrpcService::new("Svc").methods(vec!["GetUser".into(), "GetProfile".into()]);
        let request = make_request("/Svc/GetProfile");
        assert!(matches!(
            predicate.check(request).await,
            PredicateResult::Cacheable(_)
        ));
    }

    #[tokio::test]
    async fn method_in_no_match() {
        let predicate =
            GrpcService::new("Svc").methods(vec!["GetUser".into(), "GetProfile".into()]);
        let request = make_request("/Svc/DeleteUser");
        assert!(matches!(
            predicate.check(request).await,
            PredicateResult::NonCacheable(_)
        ));
    }
}
