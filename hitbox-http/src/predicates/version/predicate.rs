//! HTTP version predicate implementation.

use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};
use http::Version;

use super::operation::Operation;

/// A predicate that matches subjects by HTTP protocol version.
///
/// Works with any subject implementing [`HasVersion`], including both
/// requests and responses.
///
/// # Type Parameters
///
/// * `P` - The inner predicate to chain with. Use the `predicate()` entry point
///   from [`request`](crate::predicates::request) or [`response`](crate::predicates::response)
///   to start a new chain, then call `.version(...)`.
///
/// # Examples
///
/// ```
/// use hitbox_http::predicates::request;
/// use hitbox_http::predicates::version::{Operation, VersionPredicate};
/// use http::Version;
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// // Cache only HTTP/2 requests
/// let predicate = request::predicate::<Empty<Bytes>>()
///     .version(Operation::Eq(Version::HTTP_2));
/// ```
#[derive(Debug)]
pub struct HttpVersion<P> {
    pub(crate) operation: Operation,
    pub(crate) inner: P,
}

/// Extension trait for adding version matching to a predicate chain.
///
/// # For Callers
///
/// Chain this to match requests or responses by their HTTP protocol version.
/// Useful for applying different caching strategies based on HTTP/1.1 vs HTTP/2.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Predicate`](hitbox::predicate::Predicate)
/// types. You don't need to implement it manually.
pub trait VersionPredicate: Sized {
    /// Adds a version match to this predicate chain.
    ///
    /// Accepts an [`Operation`] or an [`http::Version`] (exact match) directly.
    fn version(self, operation: impl Into<Operation>) -> HttpVersion<Self>;
}

impl<P> VersionPredicate for P
where
    P: Predicate,
{
    fn version(self, operation: impl Into<Operation>) -> HttpVersion<Self> {
        HttpVersion {
            operation: operation.into(),
            inner: self,
        }
    }
}

/// Trait for types that provide access to the HTTP protocol version.
///
/// Implement this trait to enable version predicates on custom types.
/// Both [`CacheableHttpRequest`](crate::CacheableHttpRequest) and
/// [`CacheableHttpResponse`](crate::CacheableHttpResponse) implement this trait.
pub trait HasVersion {
    /// Returns the HTTP protocol version.
    fn http_version(&self) -> Version;
}

// Generic implementation for any subject that has an HTTP version
#[async_trait]
impl<P> Predicate for HttpVersion<P>
where
    P: Predicate + Send + Sync,
    P::Subject: HasVersion + Send,
{
    type Subject = P::Subject;
    type Context = P::Context;

    async fn check(
        &self,
        subject: Self::Subject,
        ctx: &mut Self::Context,
    ) -> PredicateResult<Self::Subject> {
        match self.inner.check(subject, ctx).await {
            PredicateResult::Cacheable(subject) => {
                if self.operation.check(subject.http_version()) {
                    PredicateResult::Cacheable(subject)
                } else {
                    PredicateResult::NonCacheable(subject)
                }
            }
            PredicateResult::NonCacheable(subject) => PredicateResult::NonCacheable(subject),
        }
    }
}
