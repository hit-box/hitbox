use crate::CacheableHttpResponse;
use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};
use http::{HeaderName, HeaderValue};
use hyper::body::Body as HttpBody;
use regex::Regex;

#[derive(Debug)]
pub enum Operation {
    Eq(HeaderName, HeaderValue),
    Exist(HeaderName),
    In(HeaderName, Vec<HeaderValue>),
    Contains(HeaderName, String),
    Regex(HeaderName, Regex),
}

#[derive(Debug)]
pub struct Header<P> {
    pub operation: Operation,
    inner: P,
}

impl<P> Header<P> {
    pub fn new(inner: P, operation: Operation) -> Self {
        Self { operation, inner }
    }
}

pub trait HeaderPredicate: Sized {
    fn header(self, operation: Operation) -> Header<Self>;
}

impl<P> HeaderPredicate for P
where
    P: Predicate,
{
    fn header(self, operation: Operation) -> Header<Self> {
        Header {
            operation,
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for Header<P>
where
    ReqBody: HttpBody + Send + 'static,
    ReqBody::Error: std::fmt::Debug,
    ReqBody::Data: Send,
    P: Predicate<Subject = CacheableHttpResponse<ReqBody>> + Send + Sync,
{
    type Subject = P::Subject;

    async fn check(&self, response: Self::Subject) -> Result<PredicateResult<Self::Subject>, hitbox::PredicateError> {
        match self.inner.check(response).await? {
            PredicateResult::Cacheable(response) => {
                let is_cacheable = match &self.operation {
                    Operation::Eq(name, value) => response
                        .parts
                        .headers
                        .get_all(name)
                        .iter()
                        .any(|header_value| value.eq(header_value)),
                    Operation::Exist(name) => response.parts.headers.get(name).is_some(),
                    Operation::In(name, values) => response
                        .parts
                        .headers
                        .get_all(name)
                        .iter()
                        .any(|header_value| values.iter().any(|v| v.eq(header_value))),
                    Operation::Contains(name, substring) => response
                        .parts
                        .headers
                        .get_all(name)
                        .iter()
                        .any(|header_value| {
                            header_value
                                .to_str()
                                .map(|s| s.contains(substring.as_str()))
                                .unwrap_or(false)
                        }),
                    Operation::Regex(name, regex) => response
                        .parts
                        .headers
                        .get_all(name)
                        .iter()
                        .any(|header_value| {
                            header_value
                                .to_str()
                                .map(|s| regex.is_match(s))
                                .unwrap_or(false)
                        }),
                };
                if is_cacheable {
                    Ok(PredicateResult::Cacheable(response))
                } else {
                    Ok(PredicateResult::NonCacheable(response))
                }
            }
            PredicateResult::NonCacheable(response) => Ok(PredicateResult::NonCacheable(response)),
        }
    }
}
