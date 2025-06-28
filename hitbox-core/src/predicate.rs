use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;

pub enum PredicateResult<S> {
    Cacheable(S),
    NonCacheable(S),
}

// @FIX: remove Debug bound for Predicate
#[async_trait]
pub trait Predicate: Debug {
    type Subject;
    async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject>;
}

#[async_trait]
impl<T> Predicate for Box<T>
where
    T: Predicate + ?Sized + Sync,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn check(&self, subject: T::Subject) -> PredicateResult<T::Subject> {
        self.as_ref().check(subject).await
    }
}

#[async_trait]
impl<T> Predicate for Arc<T>
where
    T: Predicate + Send + Sync + ?Sized,
    T::Subject: Send,
{
    type Subject = T::Subject;

    async fn check(&self, subject: T::Subject) -> PredicateResult<T::Subject> {
        self.as_ref().check(subject).await
    }
}
