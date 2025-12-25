//! Body predicate implementation.
//!
//! Provides [`Body`] predicate for matching request or response bodies.

use std::fmt::Debug;

use async_trait::async_trait;
use hitbox::Neutral;
use hitbox::predicate::{Predicate, PredicateResult};
use hyper::body::Body as HttpBody;

use super::operation::Operation;
use crate::CacheableSubject;

/// A predicate that matches HTTP bodies against an [`Operation`].
///
/// Works with both requests and responses through the [`CacheableSubject`] trait.
/// Chain with other predicates using the builder pattern.
///
/// # Type Parameters
///
/// * `P` - The inner predicate to chain with. Use [`Body::new`] to start
///   a new predicate chain (uses [`Neutral`] internally), or use the
///   [`BodyPredicate`] extension trait to chain onto an existing predicate.
///
/// # Examples
///
/// ```
/// use hitbox_http::predicates::body::{Body, Operation};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox::Neutral;
/// # use hitbox_http::CacheableHttpRequest;
/// # type Subject = CacheableHttpRequest<Empty<Bytes>>;
/// // Only cache responses smaller than 1MB
/// let predicate = Body::new(Operation::Limit { bytes: 1024 * 1024 });
/// # let _: &Body<Neutral<Subject>> = &predicate;
/// ```
///
/// # Caveats
///
/// Body predicates consume bytes from the stream. The body is buffered during
/// evaluation and returned in a [`BufferedBody`](crate::BufferedBody) state.
/// Order body predicates last in a chain when possible.
#[derive(Debug)]
pub struct Body<P> {
    pub(crate) operation: Operation,
    pub(crate) inner: P,
}

impl<S> Body<Neutral<S>> {
    /// Creates a predicate that matches body content against the operation.
    ///
    /// Returns [`Cacheable`](hitbox::predicate::PredicateResult::Cacheable) when
    /// the body satisfies the operation, [`NonCacheable`](hitbox::predicate::PredicateResult::NonCacheable) otherwise.
    ///
    /// Chain onto existing predicates using [`BodyPredicate::body`] instead
    /// if you already have a predicate chain.
    pub fn new(operation: Operation) -> Self {
        Self {
            operation,
            inner: Neutral::new(),
        }
    }
}

/// Extension trait for adding body matching to a predicate chain.
///
/// # For Callers
///
/// Chain this to add body content matching to your predicate. The body is
/// inspected and matched against the provided [`Operation`].
///
/// **Important**: Body predicates consume bytes from the stream. Place them
/// last in your predicate chain when possible.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Predicate`](hitbox::predicate::Predicate)
/// types. You don't need to implement it manually.
pub trait BodyPredicate: Sized {
    /// Adds a body matching operation to this predicate chain.
    fn body(self, operation: Operation) -> Body<Self>;
}

impl<P> BodyPredicate for P
where
    P: Predicate,
{
    fn body(self, operation: Operation) -> Body<Self> {
        Body {
            operation,
            inner: self,
        }
    }
}

// Generic implementation for any CacheableSubject
#[async_trait]
impl<P> Predicate for Body<P>
where
    P: Predicate + Send + Sync,
    P::Subject: CacheableSubject + Send,
    <P::Subject as CacheableSubject>::Body: Send + Unpin + 'static,
    <P::Subject as CacheableSubject>::Parts: Send,
    <<P::Subject as CacheableSubject>::Body as HttpBody>::Error: Debug + Send,
    <<P::Subject as CacheableSubject>::Body as HttpBody>::Data: Send,
{
    type Subject = P::Subject;

    async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject> {
        let inner_result = self.inner.check(subject).await;

        let (was_cacheable, subject) = match inner_result {
            PredicateResult::Cacheable(s) => (true, s),
            PredicateResult::NonCacheable(s) => (false, s),
        };

        let (parts, body) = subject.into_parts();
        let body_result = self.operation.check(body).await;

        // Combine: final is Cacheable only if both inner AND body are Cacheable
        match body_result {
            PredicateResult::Cacheable(buffered_body) => {
                let subject = P::Subject::from_parts(parts, buffered_body);
                if was_cacheable {
                    PredicateResult::Cacheable(subject)
                } else {
                    PredicateResult::NonCacheable(subject)
                }
            }
            PredicateResult::NonCacheable(buffered_body) => {
                PredicateResult::NonCacheable(P::Subject::from_parts(parts, buffered_body))
            }
        }
    }
}
