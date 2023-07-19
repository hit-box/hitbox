use std::sync::Arc;

use async_trait::async_trait;

pub enum PredicateResult<S> {
    Cacheable(S),
    NonCacheable(S),
}

// impl<S> PredicateResult<S>
// where
//     S: Send + 'static,
// {
//     pub async fn chain<P: Predicate + ?Sized>(self, predicate: &Box<P>) -> PredicateResult<S> {
//         dbg!("PredicateResult::chain");
//         match self {
//             PredicateResult::NonCacheable(subject) => predicate.check(subject).await,
//             PredicateResult::Cacheable(subject) => PredicateResult::Cacheable(subject),
//         }
//     }
// }

#[async_trait]
pub trait Predicate {
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

pub enum Operation {
    Eq,
    In,
    // TODO: extend predicate operations
}
