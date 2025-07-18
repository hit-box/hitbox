use crate::CacheableHttpResponse;
use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};

#[derive(Debug)]
pub struct StatusCode<P> {
    status_code: http::StatusCode,
    inner: P,
}

impl<P> StatusCode<P> {
    pub fn new(inner: P, status_code: http::StatusCode) -> Self {
        Self { status_code, inner }
    }
}

pub trait StatusCodePredicate: Sized {
    fn status_code(self, status_code: http::StatusCode) -> StatusCode<Self>;
}

impl<P> StatusCodePredicate for P
where
    P: Predicate,
{
    fn status_code(self, status_code: http::StatusCode) -> StatusCode<Self> {
        StatusCode {
            status_code,
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for StatusCode<P>
where
    P: Predicate<Subject = CacheableHttpResponse<ReqBody>> + Send + Sync,
    ReqBody: Send + 'static,
{
    type Subject = P::Subject;

    async fn check(&self, response: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(response).await {
            PredicateResult::Cacheable(response) => {
                if self.status_code == response.parts.status {
                    PredicateResult::Cacheable(response)
                } else {
                    PredicateResult::NonCacheable(response)
                }
            }
            PredicateResult::NonCacheable(response) => PredicateResult::NonCacheable(response),
        }
    }
}
