use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::predicates::{Operation, Predicate, PredicateResult};

pub struct Header {
    pub name: String,
    pub value: String,
    pub operation: Operation,
}

#[async_trait]
impl<ReqBody> Predicate<CacheableHttpRequest<ReqBody>> for Header
where
    ReqBody: Send + 'static,
{
    async fn check(
        &self,
        request: CacheableHttpRequest<ReqBody>,
    ) -> PredicateResult<CacheableHttpRequest<ReqBody>> {
        match self.operation {
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
            _ => unimplemented!(),
        }
    }
}
