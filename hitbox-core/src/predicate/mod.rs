//! Caching decision predicates.
//!
//! This module provides the [`Predicate`] trait and [`PredicateResult`] enum
//! for determining whether requests and responses should be cached.
//!
//! ## Overview
//!
//! Predicates are the core mechanism for controlling cache behavior. They
//! evaluate a subject (request or response) and return whether it should
//! be cached or passed through without caching.
//!
//! ## Composability
//!
//! Predicates are designed to be composed using logical combinators:
//!
//! - [`Not`] - Inverts a predicate result
//! - [`Or`] - Either predicate returning `Cacheable` is sufficient
//!
//! Chaining predicates sequentially provides AND semantics by default.

pub mod combinators;
pub mod neutral;

use std::sync::Arc;

use async_trait::async_trait;

use crate::EvalContext;

pub use combinators::{And, Not, Or, PredicateExt};
pub use neutral::Neutral;

/// Result of a predicate evaluation.
///
/// Indicates whether the subject should be cached or not, while preserving
/// ownership of the subject for further processing.
#[derive(Debug)]
pub enum PredicateResult<S> {
    /// Subject should be cached.
    Cacheable(S),
    /// Subject should not be cached; pass through directly.
    NonCacheable(S),
}

impl<S> PredicateResult<S> {
    /// Chains predicate checks.
    ///
    /// If `Cacheable`, applies the function which may return `Cacheable` or
    /// `NonCacheable`. If already `NonCacheable`, short-circuits and stays
    /// `NonCacheable` without calling the function.
    ///
    /// This enables predicate chaining where `NonCacheable` is "sticky":
    ///
    /// ```ignore
    /// predicate1.check(request).await
    ///     .and_then(|req| predicate2.check(req)).await
    ///     .and_then(|req| predicate3.check(req)).await
    /// ```
    pub async fn and_then<F, Fut>(self, f: F) -> PredicateResult<S>
    where
        F: FnOnce(S) -> Fut,
        Fut: std::future::Future<Output = PredicateResult<S>>,
    {
        match self {
            PredicateResult::Cacheable(value) => f(value).await,
            PredicateResult::NonCacheable(value) => PredicateResult::NonCacheable(value),
        }
    }
}

/// Trait for evaluating whether a subject should be cached.
///
/// Predicates are the core abstraction for cache decision logic. They are
/// **protocol-agnostic** - the same trait works for HTTP requests, gRPC
/// messages, or any other protocol type.
///
/// # Type Parameters
///
/// The `Subject` associated type defines what this predicate evaluates.
/// For request predicates, this is typically a request type. For response
/// predicates, this is typically a response type.
///
/// # Ownership
///
/// The `check` method takes ownership of the subject and returns it wrapped
/// in a [`PredicateResult`]. This allows the subject to flow through a chain
/// of predicates without cloning.
///
#[async_trait]
pub trait Predicate {
    /// The type being evaluated by this predicate.
    type Subject;

    /// Evaluate whether the subject should be cached.
    ///
    /// Returns [`PredicateResult::Cacheable`] if the subject should be cached,
    /// or [`PredicateResult::NonCacheable`] if it should bypass the cache.
    async fn check(
        &self,
        subject: Self::Subject,
        ctx: &EvalContext,
    ) -> PredicateResult<Self::Subject>;
}

#[async_trait]
impl<T> Predicate for Box<T>
where
    T: Predicate + ?Sized + Sync,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn check(&self, subject: T::Subject, ctx: &EvalContext) -> PredicateResult<T::Subject> {
        self.as_ref().check(subject, ctx).await
    }
}

#[async_trait]
impl<T> Predicate for &T
where
    T: Predicate + ?Sized + Sync,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn check(&self, subject: T::Subject, ctx: &EvalContext) -> PredicateResult<T::Subject> {
        (*self).check(subject, ctx).await
    }
}

#[async_trait]
impl<T> Predicate for Arc<T>
where
    T: Predicate + Send + Sync + ?Sized,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn check(&self, subject: T::Subject, ctx: &EvalContext) -> PredicateResult<T::Subject> {
        self.as_ref().check(subject, ctx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_predicate_ext_with_box_dyn() {
        let p1: Box<dyn Predicate<Subject = i32> + Send + Sync> = Box::new(Neutral::<i32>::new());
        let p2: Box<dyn Predicate<Subject = i32> + Send + Sync> = Box::new(Neutral::<i32>::new());

        // PredicateExt works on Box<dyn Predicate> because Box<T> is Sized
        let combined = p1.or(p2);

        let ctx = EvalContext::new();
        let result = combined.check(42, &ctx).await;
        assert!(matches!(result, PredicateResult::Cacheable(42)));
    }

    #[tokio::test]
    async fn test_predicate_ext_chaining_with_box_dyn() {
        let p1: Box<dyn Predicate<Subject = i32> + Send + Sync> = Box::new(Neutral::<i32>::new());
        let p2: Box<dyn Predicate<Subject = i32> + Send + Sync> = Box::new(Neutral::<i32>::new());
        let p3: Box<dyn Predicate<Subject = i32> + Send + Sync> = Box::new(Neutral::<i32>::new());

        // Chain: p1.and(p2).or(p3).not()
        let combined = p1.and(p2).or(p3).not();

        let ctx = EvalContext::new();
        let result = combined.check(42, &ctx).await;
        // Neutral returns Cacheable, so: Cacheable AND Cacheable = Cacheable, OR Cacheable = Cacheable, NOT = NonCacheable
        assert!(matches!(result, PredicateResult::NonCacheable(42)));
    }

    #[tokio::test]
    async fn test_predicate_ext_boxed() {
        // Use .boxed() for type erasure
        let p1 = Neutral::<i32>::new().boxed();
        let p2 = Neutral::<i32>::new().boxed();

        // Can chain after boxing
        let combined = p1.or(p2);

        let ctx = EvalContext::new();
        let result = combined.check(42, &ctx).await;
        assert!(matches!(result, PredicateResult::Cacheable(42)));
    }

    #[tokio::test]
    async fn test_predicate_ext_boxed_in_vec() {
        // Store heterogeneous predicates in a Vec
        let predicates: Vec<Box<dyn Predicate<Subject = i32> + Send + Sync>> = vec![
            Neutral::<i32>::new().boxed(),
            Neutral::<i32>::new().not().boxed(),
        ];

        let ctx = EvalContext::new();
        let result1 = predicates[0].check(1, &ctx).await;
        let result2 = predicates[1].check(2, &ctx).await;

        assert!(matches!(result1, PredicateResult::Cacheable(1)));
        assert!(matches!(result2, PredicateResult::NonCacheable(2)));
    }
}
