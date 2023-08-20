use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};

pub enum Operation {
    Eq,
}

pub struct Header<P> {
    pub name: String,
    pub value: String,
    pub operation: Operation,
    inner: P,
}

pub trait HeaderPredicate: Sized {
    fn header(self, name: String, value: String) -> Header<Self>;
}

impl<P> HeaderPredicate for P
where
    P: Predicate,
{
    fn header(self, name: String, value: String) -> Header<P> {
        Header {
            name,
            value,
            operation: Operation::Eq,
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for Header<P>
where
    ReqBody: Send + 'static,
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = P::Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        //dbg!("HeaderPredicate::check");
        match self.inner.check(request).await {
            PredicateResult::Cacheable(request) => match self.operation {
                Operation::Eq => match request.parts().headers.get(self.name.as_str()) {
                    Some(header_value) => {
                        if self.value.as_str() == header_value {
                            PredicateResult::Cacheable(request)
                        } else {
                            PredicateResult::NonCacheable(request)
                        }
                    }
                    None => PredicateResult::NonCacheable(request),
                },
            },
            PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
        }
    }
}
