//! gRPC status code response predicate.
//!
//! [`GrpcStatus`] checks the `grpc-status` from response trailers or headers
//! (Trailers-Only responses) to determine cacheability.

use async_trait::async_trait;
use hitbox::Neutral;
use hitbox::predicate::{Predicate, PredicateResult};
use hitbox_http::CacheableHttpResponse;

use crate::code::GrpcCode;

/// A predicate that matches responses by their gRPC status code.
///
/// gRPC status is typically sent in HTTP/2 trailers. For Trailers-Only responses
/// (errors), it's in the response headers instead. This predicate checks both.
///
/// # Examples
///
/// ```ignore
/// use hitbox_grpc::predicates::status::GrpcStatus;
/// use hitbox_grpc::code::GrpcCode;
///
/// // Only cache successful responses
/// let predicate = GrpcStatus::ok();
///
/// // Cache OK or NOT_FOUND responses
/// let predicate = GrpcStatus::new_in(vec![GrpcCode::Ok, GrpcCode::NotFound]);
/// ```
#[derive(Debug)]
pub struct GrpcStatus<P> {
    codes: Vec<GrpcCode>,
    inner: P,
}

impl<S> GrpcStatus<Neutral<S>> {
    /// Creates a predicate that matches only `OK` (code 0) responses.
    pub fn ok() -> Self {
        GrpcStatus {
            codes: vec![GrpcCode::Ok],
            inner: Neutral::new(),
        }
    }

    /// Creates a predicate matching any of the specified gRPC status codes.
    pub fn new_in(codes: Vec<GrpcCode>) -> Self {
        GrpcStatus {
            codes,
            inner: Neutral::new(),
        }
    }
}

/// Extension trait for adding gRPC status matching to a response predicate chain.
pub trait GrpcStatusPredicate: Sized {
    /// Adds gRPC status code matching to this predicate chain.
    fn grpc_status_ok(self) -> GrpcStatus<Self>;
}

impl<P> GrpcStatusPredicate for P
where
    P: Predicate,
{
    fn grpc_status_ok(self) -> GrpcStatus<Self> {
        GrpcStatus {
            codes: vec![GrpcCode::Ok],
            inner: self,
        }
    }
}

/// Extract grpc-status from headers (used for both response headers and trailers).
fn extract_grpc_status(headers: &http::HeaderMap) -> Option<GrpcCode> {
    headers
        .get("grpc-status")
        .and_then(|v| GrpcCode::from_bytes(v.as_bytes()))
}

#[async_trait]
impl<P, ResBody> Predicate for GrpcStatus<P>
where
    P: Predicate<Subject = CacheableHttpResponse<ResBody>> + Send + Sync,
    ResBody: hyper::body::Body + Send + 'static,
    ResBody::Error: Send,
{
    type Subject = P::Subject;

    async fn check(&self, response: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(response).await {
            PredicateResult::Cacheable(response) => {
                // Check grpc-status in response headers (Trailers-Only format)
                let status = extract_grpc_status(&response.parts.headers);
                let matches = status.is_some_and(|code| self.codes.contains(&code));

                if matches {
                    PredicateResult::Cacheable(response)
                } else {
                    PredicateResult::NonCacheable(response)
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

    fn make_response(grpc_status: &str) -> CacheableHttpResponse<Full<Bytes>> {
        let response = http::Response::builder()
            .status(200)
            .header("content-type", "application/grpc")
            .header("grpc-status", grpc_status)
            .body(hitbox_http::BufferedBody::Passthrough(Full::new(
                Bytes::new(),
            )))
            .unwrap();
        CacheableHttpResponse::from_response(response)
    }

    #[tokio::test]
    async fn ok_matches_status_0() {
        let predicate = GrpcStatus::ok();
        let response = make_response("0");
        assert!(matches!(
            predicate.check(response).await,
            PredicateResult::Cacheable(_)
        ));
    }

    #[tokio::test]
    async fn ok_rejects_error_status() {
        let predicate = GrpcStatus::ok();
        let response = make_response("13"); // INTERNAL
        assert!(matches!(
            predicate.check(response).await,
            PredicateResult::NonCacheable(_)
        ));
    }

    #[tokio::test]
    async fn new_in_matches() {
        let predicate = GrpcStatus::new_in(vec![GrpcCode::Ok, GrpcCode::NotFound]);
        let response = make_response("5"); // NOT_FOUND
        assert!(matches!(
            predicate.check(response).await,
            PredicateResult::Cacheable(_)
        ));
    }

    #[tokio::test]
    async fn missing_grpc_status_is_non_cacheable() {
        let predicate = GrpcStatus::ok();
        let response = http::Response::builder()
            .status(200)
            .body(hitbox_http::BufferedBody::Passthrough(Full::new(
                Bytes::new(),
            )))
            .unwrap();
        let response = CacheableHttpResponse::from_response(response);
        assert!(matches!(
            predicate.check(response).await,
            PredicateResult::NonCacheable(_)
        ));
    }
}
