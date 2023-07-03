use async_trait::async_trait;

pub enum PredicateResult<S> {
    Cacheable(S),
    NonCacheable(S),
}

impl<S> PredicateResult<S> {
    pub async fn chain<P: Predicate<S>>(self, predicate: &P) -> PredicateResult<S> {
        match self {
            PredicateResult::NonCacheable(subject) => predicate.check(subject).await,
            PredicateResult::Cacheable(subject) => PredicateResult::Cacheable(subject),
        }
    }
}

#[async_trait]
pub trait Predicate<S> {
    async fn check(&self, subject: S) -> PredicateResult<S>;
}

#[async_trait]
impl<S, T> Predicate<S> for Box<T>
where
    T: Predicate<S> + Sync,
    S: Send + 'static,
{
    async fn check(&self, subject: S) -> PredicateResult<S> {
        self.as_ref().check(subject).await
    }
}

pub enum Operation {
    Eq,
    In,
    // TODO: extend predicate operations
}
