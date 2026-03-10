//! Logical combinators for composing predicates.
//!
//! This module provides generic combinators that work with any [`Predicate`]
//! implementation, regardless of the protocol.
//!
//! ## Extension Trait
//!
//! The [`PredicateExt`] trait provides fluent methods for predicate composition:
//!
//! ```ignore
//! use hitbox_core::predicate::{Neutral, PredicateExt};
//!
//! let predicate = Neutral::new()
//!     .and(predicate1)
//!     .or(predicate2)
//!     .not();
//! ```

use async_trait::async_trait;

use super::{Predicate, PredicateResult};

/// Inverts a predicate result.
///
/// - `Cacheable` becomes `NonCacheable`
/// - `NonCacheable` becomes `Cacheable`
#[derive(Debug)]
pub struct Not<P> {
    predicate: P,
}

impl<P> Not<P> {
    /// Creates a new `Not` combinator wrapping the given predicate.
    pub fn new(predicate: P) -> Self {
        Self { predicate }
    }
}

#[async_trait]
impl<P> Predicate for Not<P>
where
    P: Predicate + Send + Sync,
    P::Subject: Send,
    P::Context: Send,
{
    type Subject = P::Subject;
    type Context = P::Context;

    async fn check(
        &self,
        subject: Self::Subject,
        ctx: &mut Self::Context,
    ) -> PredicateResult<Self::Subject> {
        match self.predicate.check(subject, ctx).await {
            PredicateResult::Cacheable(s) => PredicateResult::NonCacheable(s),
            PredicateResult::NonCacheable(s) => PredicateResult::Cacheable(s),
        }
    }
}

/// Requires both predicates to return `Cacheable`.
///
/// Short-circuits: if the left predicate returns `NonCacheable`,
/// the right predicate is not evaluated.
#[derive(Debug)]
pub struct And<L, R> {
    left: L,
    right: R,
}

impl<L, R> And<L, R> {
    /// Creates a new `And` combinator from two predicates.
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

#[async_trait]
impl<L, R> Predicate for And<L, R>
where
    L: Predicate + Send + Sync,
    R: Predicate<Subject = L::Subject, Context = L::Context> + Send + Sync,
    L::Subject: Send,
    L::Context: Send,
{
    type Subject = L::Subject;
    type Context = L::Context;

    async fn check(
        &self,
        subject: Self::Subject,
        ctx: &mut Self::Context,
    ) -> PredicateResult<Self::Subject> {
        match self.left.check(subject, ctx).await {
            PredicateResult::Cacheable(s) => self.right.check(s, ctx).await,
            non_cacheable => non_cacheable,
        }
    }
}

/// Requires either predicate to return `Cacheable`.
///
/// Short-circuits: if the left predicate returns `Cacheable`,
/// the right predicate is not evaluated.
#[derive(Debug)]
pub struct Or<L, R> {
    left: L,
    right: R,
}

impl<L, R> Or<L, R> {
    /// Creates a new `Or` combinator from two predicates.
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

#[async_trait]
impl<L, R> Predicate for Or<L, R>
where
    L: Predicate + Send + Sync,
    R: Predicate<Subject = L::Subject, Context = L::Context> + Send + Sync,
    L::Subject: Send,
    L::Context: Send,
{
    type Subject = L::Subject;
    type Context = L::Context;

    async fn check(
        &self,
        subject: Self::Subject,
        ctx: &mut Self::Context,
    ) -> PredicateResult<Self::Subject> {
        match self.left.check(subject, ctx).await {
            PredicateResult::NonCacheable(s) => self.right.check(s, ctx).await,
            cacheable => cacheable,
        }
    }
}

/// Extension trait for fluent predicate composition.
pub trait PredicateExt: Predicate + Sized {
    /// Combines this predicate with another using AND logic.
    ///
    /// Returns `Cacheable` only if both predicates return `Cacheable`.
    fn and<R>(self, right: R) -> And<Self, R>
    where
        R: Predicate<Subject = Self::Subject, Context = Self::Context>,
    {
        And::new(self, right)
    }

    /// Combines this predicate with another using OR logic.
    ///
    /// Returns `Cacheable` if either predicate returns `Cacheable`.
    fn or<R>(self, right: R) -> Or<Self, R>
    where
        R: Predicate<Subject = Self::Subject, Context = Self::Context>,
    {
        Or::new(self, right)
    }

    /// Inverts this predicate's result.
    ///
    /// `Cacheable` becomes `NonCacheable` and vice versa.
    fn not(self) -> Not<Self> {
        Not::new(self)
    }

    /// Boxes this predicate into a trait object.
    ///
    /// Useful for type erasure when storing predicates in collections
    /// or returning them from functions.
    fn boxed(
        self,
    ) -> Box<dyn Predicate<Subject = Self::Subject, Context = Self::Context> + Send + Sync>
    where
        Self: Send + Sync + 'static,
    {
        Box::new(self)
    }
}

impl<T: Predicate + Sized> PredicateExt for T {}
