use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;
use thiserror::Error;

pub enum PredicateResult<S> {
    Cacheable(S),
    NonCacheable(S),
}

#[derive(Debug, Error)]
pub enum PredicateError {
    /// Failed to collect or read HTTP body
    #[error("Failed to collect HTTP body: {0}")]
    BodyCollection(Box<dyn std::error::Error + Send>),

    /// Internal predicate evaluation error
    #[error("Internal predicate error: {0}")]
    Internal(Box<dyn std::error::Error + Send>),
}

// @FIX: remove Debug bound for Predicate
#[async_trait]
pub trait Predicate: Debug {
    type Subject;
    async fn check(
        &self,
        subject: Self::Subject,
    ) -> Result<PredicateResult<Self::Subject>, PredicateError>;
}

#[async_trait]
impl<T> Predicate for Box<T>
where
    T: Predicate + ?Sized + Sync,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn check(
        &self,
        subject: T::Subject,
    ) -> Result<PredicateResult<T::Subject>, PredicateError> {
        self.as_ref().check(subject).await
    }
}

#[async_trait]
impl<T> Predicate for &T
where
    T: Predicate + ?Sized + Sync,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn check(
        &self,
        subject: T::Subject,
    ) -> Result<PredicateResult<T::Subject>, PredicateError> {
        (*self).check(subject).await
    }
}

#[async_trait]
impl<T> Predicate for Arc<T>
where
    T: Predicate + Send + Sync + ?Sized,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn check(
        &self,
        subject: T::Subject,
    ) -> Result<PredicateResult<T::Subject>, PredicateError> {
        self.as_ref().check(subject).await
    }
}
