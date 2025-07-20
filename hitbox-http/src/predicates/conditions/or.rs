use async_trait::async_trait;
use hitbox::predicate::PredicateResult;
use hitbox::Predicate;

#[derive(Debug)]
pub struct Or<L, R> {
    left: L,
    right: R,
}

impl<L, R> Or<L, R> {
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

#[async_trait]
impl<L, R, Subject> Predicate for Or<L, R>
where
    Subject: Send + 'static,
    L: Predicate<Subject = Subject> + Send + Sync,
    R: Predicate<Subject = Subject> + Send + Sync,
{
    type Subject = Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        let left = self.left.check(request).await;
        match left {
            PredicateResult::Cacheable(request) => PredicateResult::Cacheable(request),
            PredicateResult::NonCacheable(request) => match self.right.check(request).await {
                PredicateResult::Cacheable(request) => PredicateResult::Cacheable(request),
                PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
            },
        }
    }
}

pub trait OrPredicate: Sized {
    fn or<P: Predicate>(self, predicate: P) -> Or<Self, P>;
}

impl<T> OrPredicate for T
where
    T: Predicate,
{
    fn or<P>(self, predicate: P) -> Or<Self, P> {
        Or {
            left: self,
            right: predicate,
        }
    }
}
