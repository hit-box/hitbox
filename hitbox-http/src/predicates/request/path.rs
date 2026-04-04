//! Path pattern matching predicate.
//!
//! Provides [`Path`] predicate and [`Operation`] for matching request paths against
//! [actix-router](https://docs.rs/actix-router) patterns.

use crate::CacheableHttpRequest;
use actix_router::ResourceDef;
use async_trait::async_trait;
use hitbox::EvalContext;
use hitbox::predicate::{Predicate, PredicateResult};

/// Matching operations for request paths.
///
/// # Pattern Syntax
///
/// - `{name}` — matches a path segment
/// - `{name:regex}` — matches with regex constraint
/// - `{tail}*` — matches remaining path segments
#[derive(Debug)]
pub enum Operation {
    /// Match a path against a pattern.
    Pattern(ResourceDef),
}

impl Operation {
    /// Match a path against a pattern.
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_http::predicates::request::path::Operation;
    ///
    /// let op = Operation::pattern("/api/users/{id}");
    /// ```
    pub fn pattern(value: impl Into<String>) -> Self {
        Operation::Pattern(ResourceDef::new(value.into()))
    }
}

impl From<&str> for Operation {
    fn from(pattern: &str) -> Self {
        Operation::pattern(pattern)
    }
}

impl From<String> for Operation {
    fn from(pattern: String) -> Self {
        Operation::pattern(pattern)
    }
}

/// A predicate that matches requests by path pattern.
///
/// Returns [`Cacheable`](PredicateResult::Cacheable) when the request path
/// matches the pattern, [`NonCacheable`](PredicateResult::NonCacheable) otherwise.
///
/// # Type Parameters
///
/// * `P` - The inner predicate to chain with. Use [`request::predicate()`](super::predicate)
///   to start a new chain, then call `.path(...)`.
///
/// # Examples
///
/// ```
/// use hitbox_http::predicates::request::{self, PathPredicate};
/// use hitbox_http::predicates::request::path::Operation;
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// // Match requests to /api/users/{id}
/// let predicate = request::predicate::<Empty<Bytes>>()
///     .path(Operation::pattern("/api/users/{id}"));
/// ```
#[derive(Debug)]
pub struct Path<P> {
    pub(crate) operation: Operation,
    pub(crate) inner: P,
}

/// Extension trait for adding path matching to a predicate chain.
///
/// # For Callers
///
/// Chain this to match requests by their URL path. The path is matched
/// against an [actix-router](https://docs.rs/actix-router) pattern supporting
/// dynamic segments like `{id}` and wildcards like `{tail}*`.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Predicate`]
/// types. You don't need to implement it manually.
pub trait PathPredicate: Sized {
    /// Adds a path pattern match to this predicate chain.
    ///
    /// Accepts an [`Operation`] or a string pattern directly.
    fn path(self, operation: impl Into<Operation>) -> Path<Self>;
}

impl<P> PathPredicate for P
where
    P: Predicate,
{
    fn path(self, operation: impl Into<Operation>) -> Path<Self> {
        Path {
            operation: operation.into(),
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for Path<P>
where
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
{
    type Subject = P::Subject;

    async fn check(
        &self,
        request: Self::Subject,
        ctx: &mut EvalContext,
    ) -> PredicateResult<Self::Subject> {
        match self.inner.check(request, ctx).await {
            PredicateResult::Cacheable(request) => {
                let is_match = match &self.operation {
                    Operation::Pattern(resource) => resource.is_match(request.parts().uri.path()),
                };
                if is_match {
                    PredicateResult::Cacheable(request)
                } else {
                    PredicateResult::NonCacheable(request)
                }
            }
            PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
        }
    }
}
