use crate::CacheableHttpResponse;
use async_trait::async_trait;
use hitbox::predicate::{Predicate, PredicateResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StatusClass {
    Informational, // 1xx (100-199)
    Success,       // 2xx (200-299)
    Redirect,      // 3xx (300-399)
    ClientError,   // 4xx (400-499)
    ServerError,   // 5xx (500-599)
}

impl StatusClass {
    fn matches(&self, code: http::StatusCode) -> bool {
        match self {
            StatusClass::Informational => code.is_informational(),
            StatusClass::Success => code.is_success(),
            StatusClass::Redirect => code.is_redirection(),
            StatusClass::ClientError => code.is_client_error(),
            StatusClass::ServerError => code.is_server_error(),
        }
    }
}

#[derive(Debug)]
pub enum Operation {
    Eq(http::StatusCode),
    In(Vec<http::StatusCode>),
    Range(http::StatusCode, http::StatusCode),
    Class(StatusClass),
}

impl Operation {
    fn matches(&self, status: http::StatusCode) -> bool {
        match self {
            Operation::Eq(expected) => status == *expected,
            Operation::In(codes) => codes.contains(&status),
            Operation::Range(start, end) => {
                status.as_u16() >= start.as_u16() && status.as_u16() <= end.as_u16()
            }
            Operation::Class(class) => class.matches(status),
        }
    }
}

#[derive(Debug)]
pub struct StatusCode<P> {
    operation: Operation,
    inner: P,
}

impl<P> StatusCode<P> {
    pub fn new(inner: P, status_code: http::StatusCode) -> Self {
        Self {
            operation: Operation::Eq(status_code),
            inner,
        }
    }

    pub fn new_in(inner: P, codes: Vec<http::StatusCode>) -> Self {
        Self {
            operation: Operation::In(codes),
            inner,
        }
    }

    pub fn new_range(inner: P, start: http::StatusCode, end: http::StatusCode) -> Self {
        Self {
            operation: Operation::Range(start, end),
            inner,
        }
    }

    pub fn new_class(inner: P, class: StatusClass) -> Self {
        Self {
            operation: Operation::Class(class),
            inner,
        }
    }
}

pub trait StatusCodePredicate: Sized {
    fn status_code(self, status_code: http::StatusCode) -> StatusCode<Self>;
    fn status_code_in(self, codes: Vec<http::StatusCode>) -> StatusCode<Self>;
    fn status_code_range(self, start: http::StatusCode, end: http::StatusCode) -> StatusCode<Self>;
    fn status_code_class(self, class: StatusClass) -> StatusCode<Self>;
}

impl<P> StatusCodePredicate for P
where
    P: Predicate,
{
    fn status_code(self, status_code: http::StatusCode) -> StatusCode<Self> {
        StatusCode::new(self, status_code)
    }

    fn status_code_in(self, codes: Vec<http::StatusCode>) -> StatusCode<Self> {
        StatusCode::new_in(self, codes)
    }

    fn status_code_range(self, start: http::StatusCode, end: http::StatusCode) -> StatusCode<Self> {
        StatusCode::new_range(self, start, end)
    }

    fn status_code_class(self, class: StatusClass) -> StatusCode<Self> {
        StatusCode::new_class(self, class)
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for StatusCode<P>
where
    P: Predicate<Subject = CacheableHttpResponse<ReqBody>> + Send + Sync,
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
{
    type Subject = P::Subject;

    async fn check(&self, response: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(response).await {
            PredicateResult::Cacheable(response) => {
                if self.operation.matches(response.parts.status) {
                    PredicateResult::Cacheable(response)
                } else {
                    PredicateResult::NonCacheable(response)
                }
            }
            PredicateResult::NonCacheable(response) => PredicateResult::NonCacheable(response),
        }
    }
}
