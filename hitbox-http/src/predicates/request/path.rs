//! Path pattern matching predicate.
//!
//! Provides [`Path`] predicate for matching request paths against
//! [actix-router](https://docs.rs/actix-router) patterns.

use crate::CacheableHttpRequest;
use actix_router::ResourceDef;
use async_trait::async_trait;
use hitbox::Neutral;
use hitbox::predicate::{Predicate, PredicateResult};

/// A predicate that matches request paths against a pattern.
///
/// Uses [actix-router](https://docs.rs/actix-router) for pattern matching.
/// The predicate returns [`Cacheable`](PredicateResult::Cacheable) when the
/// request path matches the pattern.
///
/// # Type Parameters
///
/// * `P` - The inner predicate to chain with. Use [`Path::new`] to start
///   a new predicate chain (uses [`Neutral`] internally), or use the
///   [`PathPredicate`] extension trait to chain onto an existing predicate.
///
/// # Pattern Syntax
///
/// - `{name}` — matches a path segment
/// - `{name:regex}` — matches with regex constraint
/// - `{tail}*` — matches remaining path segments
///
/// # Examples
///
/// ```
/// use hitbox_http::predicates::request::{Path, PathPredicate};
/// use actix_router::ResourceDef;
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox::Neutral;
/// # use hitbox_http::CacheableHttpRequest;
/// # type Subject = CacheableHttpRequest<Empty<Bytes>>;
/// // Match requests to /api/users/{id}
/// let predicate = Path::new(ResourceDef::new("/api/users/{id}"));
/// # let _: &Path<Neutral<Subject>> = &predicate;
/// ```
#[derive(Debug)]
pub struct Path<P> {
    resource: ResourceDef,
    inner: P,
}

impl<S> Path<Neutral<S>> {
    /// Creates a predicate that matches request paths against a pattern.
    ///
    /// Returns [`Cacheable`](hitbox::predicate::PredicateResult::Cacheable) when
    /// the request path matches the pattern, [`NonCacheable`](hitbox::predicate::PredicateResult::NonCacheable) otherwise.
    ///
    /// Chain onto existing predicates using [`PathPredicate::path`] instead
    /// if you already have a predicate chain.
    pub fn new(resource: ResourceDef) -> Self {
        Self {
            resource,
            inner: Neutral::new(),
        }
    }
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
    /// The pattern is compiled into a [`ResourceDef`].
    fn path(self, resource: String) -> Path<Self>;
}

impl<P> PathPredicate for P
where
    P: Predicate,
{
    fn path(self, resource: String) -> Path<Self> {
        Path {
            resource: ResourceDef::from(resource),
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

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(request).await {
            PredicateResult::Cacheable(request) => {
                if self.resource.is_match(request.parts().uri.path()) {
                    PredicateResult::Cacheable(request)
                } else {
                    PredicateResult::NonCacheable(request)
                }
            }
            PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
        }
    }
}
