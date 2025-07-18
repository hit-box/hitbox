use async_trait::async_trait;
use hitbox::predicate::PredicateResult;
use hitbox::Predicate;

#[derive(Debug)]
pub struct And<L, R> {
    left: L,
    right: R,
}

impl<L, R> And<L, R> {
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

#[async_trait]
impl<L, R, Subject> Predicate for And<L, R>
where
    Subject: Send + 'static,
    L: Predicate<Subject = Subject> + Send + Sync,
    R: Predicate<Subject = Subject> + Send + Sync,
{
    type Subject = Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        let left = self.left.check(request).await;
        match left {
            PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
            PredicateResult::Cacheable(request) => self.right.check(request).await,
        }
    }
}

pub trait AndPredicate: Sized {
    fn and<P: Predicate>(self, predicate: P) -> And<Self, P>;
}

impl<T> AndPredicate for T
where
    T: Predicate,
{
    fn and<P>(self, predicate: P) -> And<Self, P> {
        And {
            left: self,
            right: predicate,
        }
    }
}
