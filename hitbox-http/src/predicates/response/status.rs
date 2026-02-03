use crate::CacheableHttpResponse;
use async_trait::async_trait;
use hitbox::Neutral;
use hitbox::predicate::{Predicate, PredicateResult};

/// HTTP status code classes for broad matching.
///
/// Use this to match entire categories of responses instead of specific codes.
///
/// # Examples
///
/// ```
/// use hitbox_http::predicates::response::StatusClass;
///
/// // Match any 2xx response
/// let class = StatusClass::Success;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StatusClass {
    /// 1xx (100-199): Informational responses.
    Informational,
    /// 2xx (200-299): Successful responses.
    Success,
    /// 3xx (300-399): Redirection responses.
    Redirect,
    /// 4xx (400-499): Client error responses.
    ClientError,
    /// 5xx (500-599): Server error responses.
    ServerError,
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

/// Matching operations for HTTP status codes.
///
/// # Variants
///
/// - [`Eq`](Self::Eq): Matches exactly one status code
/// - [`In`](Self::In): Matches any code in the provided list
/// - [`Range`](Self::Range): Matches codes within an inclusive range
/// - [`Class`](Self::Class): Matches all codes in a status class (1xx, 2xx, etc.)
#[derive(Debug)]
pub enum Operation {
    /// Match a specific status code.
    Eq(http::StatusCode),
    /// Match any of the specified status codes.
    In(Vec<http::StatusCode>),
    /// Match status codes within a range (inclusive).
    Range(http::StatusCode, http::StatusCode),
    /// Match all status codes in a class (e.g., all 2xx).
    Class(StatusClass),
}

impl Operation {
    /// Match a specific status code.
    pub fn eq(status_code: http::StatusCode) -> Self {
        Operation::Eq(status_code)
    }

    /// Match any of the specified status codes.
    pub fn any(codes: Vec<http::StatusCode>) -> Self {
        Operation::In(codes)
    }

    /// Match status codes within a range (inclusive).
    pub fn range(start: http::StatusCode, end: http::StatusCode) -> Self {
        Operation::Range(start, end)
    }

    /// Match all status codes in a class (e.g., all 2xx).
    pub fn class(class: StatusClass) -> Self {
        Operation::Class(class)
    }
}

impl From<http::StatusCode> for Operation {
    /// Shorthand for `Operation::eq(code)`.
    fn from(code: http::StatusCode) -> Self {
        Operation::Eq(code)
    }
}

impl From<StatusClass> for Operation {
    /// Shorthand for `Operation::class(class)`.
    fn from(class: StatusClass) -> Self {
        Operation::Class(class)
    }
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

/// A predicate that matches responses by HTTP status code.
///
/// # Type Parameters
///
/// * `P` - The inner predicate to chain with. Use [`StatusCode::new`] to start
///   a new predicate chain (uses [`Neutral`] internally), or use the
///   [`StatusCodePredicate`] extension trait to chain onto an existing predicate.
///
/// # Examples
///
/// Match only 200 OK responses:
///
/// ```
/// use hitbox_http::predicates::response::status::{StatusCode, Operation};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox::Neutral;
/// # use hitbox_http::CacheableHttpResponse;
/// # type Subject = CacheableHttpResponse<Empty<Bytes>>;
/// let predicate = StatusCode::new(Operation::eq(http::StatusCode::OK));
/// # let _: &StatusCode<Neutral<Subject>> = &predicate;
/// ```
///
/// Chain with body predicate:
///
/// ```
/// use hitbox_http::predicates::response::status::{StatusCode, Operation};
/// use hitbox_http::predicates::body::{BodyPredicate, Operation as BodyOperation, PlainOperation};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox::Neutral;
/// # use hitbox_http::CacheableHttpResponse;
/// # use hitbox_http::predicates::body::Body;
/// # type Subject = CacheableHttpResponse<Empty<Bytes>>;
/// let predicate = StatusCode::new(Operation::eq(http::StatusCode::OK))
///     .body(BodyOperation::Plain(PlainOperation::Contains("success".into())));
/// # let _: &Body<StatusCode<Neutral<Subject>>> = &predicate;
/// ```
#[derive(Debug)]
pub struct StatusCode<P> {
    pub(crate) operation: Operation,
    pub(crate) inner: P,
}

impl<S> StatusCode<Neutral<S>> {
    /// Creates a standalone status code predicate from an [`Operation`].
    ///
    /// For chaining, use the [`StatusCodePredicate`] extension trait directly.
    pub fn new(operation: Operation) -> Self {
        Self {
            operation,
            inner: Neutral::new(),
        }
    }
}

/// Extension trait for adding status code matching to a predicate chain.
///
/// # For Callers
///
/// Chain these methods to match responses by their HTTP status code.
/// Use `status_code` for exact matches, `status_code_class` for broad
/// categories (like "all 2xx"), or `status_code_in`/`status_code_range`
/// for flexible matching.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Predicate`]
/// types. You don't need to implement it manually.
pub trait StatusCodePredicate: Sized {
    /// Adds a status code match to this predicate chain.
    ///
    /// Accepts an [`Operation`], an [`http::StatusCode`] (exact match),
    /// or a [`StatusClass`] (class match) directly.
    fn status(self, operation: impl Into<Operation>) -> StatusCode<Self>;
}

impl<P> StatusCodePredicate for P
where
    P: Predicate,
{
    fn status(self, operation: impl Into<Operation>) -> StatusCode<Self> {
        StatusCode {
            operation: operation.into(),
            inner: self,
        }
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
