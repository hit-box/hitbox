//! Header predicate implementation.

use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};
use http::HeaderMap;

use super::operation::Operation;

/// A predicate that matches HTTP headers against an [`Operation`].
///
/// Works with any subject implementing [`HasHeaders`], including both
/// requests and responses. Chain with other predicates using the builder pattern.
///
/// # Type Parameters
///
/// * `P` - The inner predicate to chain with. Use the `predicate()` entry point
///   from [`request`](crate::predicates::request) or [`response`](crate::predicates::response)
///   to start a new chain, then call `.header(...)`.
///
/// # Examples
///
/// ```
/// use hitbox_http::predicates::header::{HeaderPredicate, Operation};
/// use hitbox_http::predicates::request;
/// use http::header::CACHE_CONTROL;
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// // Skip caching when Cache-Control contains "no-cache"
/// let predicate = request::predicate::<Empty<Bytes>>()
///     .header(Operation::Contains(
///         CACHE_CONTROL,
///         "no-cache".to_string(),
///     ));
/// ```
#[derive(Debug)]
pub struct Header<P> {
    pub(crate) operation: Operation,
    pub(crate) inner: P,
}

/// Extension trait for adding header matching to a predicate chain.
///
/// # For Callers
///
/// Chain this to add header matching to your predicate. The request or
/// response headers are inspected and matched against the provided [`Operation`].
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Predicate`](hitbox::predicate::Predicate)
/// types. You don't need to implement it manually.
pub trait HeaderPredicate: Sized {
    /// Adds a header matching operation to this predicate chain.
    ///
    /// Accepts an [`Operation`], a [`HeaderName`](http::HeaderName) (existence check),
    /// or `(HeaderName, HeaderValue)` (exact match) directly.
    fn header(self, operation: impl Into<Operation>) -> Header<Self>;
}

impl<P> HeaderPredicate for P
where
    P: Predicate,
{
    fn header(self, operation: impl Into<Operation>) -> Header<Self> {
        Header {
            operation: operation.into(),
            inner: self,
        }
    }
}

/// Trait for types that provide access to HTTP headers.
///
/// Implement this trait to enable header predicates on custom types.
/// Both [`CacheableHttpRequest`](crate::CacheableHttpRequest) and
/// [`CacheableHttpResponse`](crate::CacheableHttpResponse) implement this trait.
///
/// # For Implementors
///
/// Return a reference to the headers associated with the HTTP message.
/// The returned headers should reflect the current state of the message
/// and remain valid for the lifetime of the borrow.
///
/// # For Callers
///
/// Use this trait to access headers generically from either requests or
/// responses. Header predicates use this to inspect headers without knowing
/// the concrete message type.
pub trait HasHeaders {
    /// Returns a reference to the HTTP headers.
    fn headers(&self) -> &HeaderMap;
}

// Generic implementation for any subject that has headers
#[async_trait]
impl<P> Predicate for Header<P>
where
    P: Predicate + Send + Sync,
    P::Subject: HasHeaders + Send,
{
    type Subject = P::Subject;

    async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(subject).await {
            PredicateResult::Cacheable(subject) => {
                let is_cacheable = self.operation.check(subject.headers());
                if is_cacheable {
                    PredicateResult::Cacheable(subject)
                } else {
                    PredicateResult::NonCacheable(subject)
                }
            }
            PredicateResult::NonCacheable(subject) => PredicateResult::NonCacheable(subject),
        }
    }
}
