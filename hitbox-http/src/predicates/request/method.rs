use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};

#[derive(Debug)]
pub enum Operation {
    Eq(http::Method),
    In(Vec<http::Method>),
}

#[derive(Debug)]
pub struct Method<P> {
    operation: Operation,
    inner: P,
}

impl<P> Method<P> {
    pub fn new<E, T>(inner: P, method: T) -> Result<Self, E>
    where
        T: TryInto<http::Method, Error = E>,
    {
        Ok(Method {
            operation: Operation::Eq(method.try_into()?),
            inner,
        })
    }

    pub fn new_in(inner: P, methods: Vec<http::Method>) -> Self {
        Method {
            operation: Operation::In(methods),
            inner,
        }
    }
}

pub trait MethodPredicate: Sized {
    fn method(self, method: http::Method) -> Method<Self>;
}

impl<P> MethodPredicate for P
where
    P: Predicate,
{
    fn method(self, method: http::Method) -> Method<Self> {
        Method {
            operation: Operation::Eq(method),
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for Method<P>
where
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
    ReqBody: Send + 'static,
{
    type Subject = P::Subject;

    async fn check(
        &self,
        request: Self::Subject,
    ) -> Result<PredicateResult<Self::Subject>, hitbox::PredicateError> {
        match self.inner.check(request).await? {
            PredicateResult::Cacheable(request) => {
                let is_cacheable = match &self.operation {
                    Operation::Eq(method) => *method == request.parts().method,
                    Operation::In(methods) => methods.contains(&request.parts().method),
                };
                if is_cacheable {
                    Ok(PredicateResult::Cacheable(request))
                } else {
                    Ok(PredicateResult::NonCacheable(request))
                }
            }
            PredicateResult::NonCacheable(request) => Ok(PredicateResult::NonCacheable(request)),
        }
    }
}
