use async_trait::async_trait;
use hitbox::Predicate;
use hitbox::predicate::PredicateResult;

#[derive(Debug)]
pub struct Or<L, R, P> {
    left: L,
    right: R,
    inner: P,
}

impl<L, R, P> Or<L, R, P> {
    pub fn new(left: L, right: R, inner: P) -> Self {
        Self { left, right, inner }
    }
}

#[async_trait]
impl<L, R, P, Subject> Predicate for Or<L, R, P>
where
    Subject: Send + 'static,
    P: Predicate<Subject = Subject> + Send + Sync,
    L: Predicate<Subject = Subject> + Send + Sync,
    R: Predicate<Subject = Subject> + Send + Sync,
{
    type Subject = Subject;

    async fn check(
        &self,
        subject: Self::Subject,
    ) -> Result<PredicateResult<Self::Subject>, hitbox::PredicateError> {
        let inner_result = self.inner.check(subject).await?;
        match inner_result {
            PredicateResult::Cacheable(subject) => {
                let left = self.left.check(subject).await?;
                match left {
                    PredicateResult::Cacheable(request) => Ok(PredicateResult::Cacheable(request)),
                    PredicateResult::NonCacheable(request) => self.right.check(request).await,
                }
            }
            PredicateResult::NonCacheable(_) => Ok(inner_result),
        }
    }
}

pub trait OrPredicate: Sized {
    fn or<L: Predicate, R: Predicate>(self, left: L, right: R) -> Or<L, R, Self>;
}

impl<T> OrPredicate for T
where
    T: Predicate,
{
    fn or<L, R>(self, left: L, right: R) -> Or<L, R, Self> {
        Or {
            left,
            right,
            inner: self,
        }
    }
}
