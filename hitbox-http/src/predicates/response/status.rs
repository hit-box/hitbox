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
/// use hitbox_http::predicates::response::StatusCode;
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox::Neutral;
/// # use hitbox_http::CacheableHttpResponse;
/// # type Subject = CacheableHttpResponse<Empty<Bytes>>;
/// let predicate = StatusCode::new(http::StatusCode::OK);
/// # let _: &StatusCode<Neutral<Subject>> = &predicate;
/// ```
///
/// Chain with body predicate:
///
/// ```
/// use hitbox_http::predicates::response::StatusCode;
/// use hitbox_http::predicates::body::{BodyPredicate, Operation as BodyOperation, PlainOperation};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox::Neutral;
/// # use hitbox_http::CacheableHttpResponse;
/// # use hitbox_http::predicates::body::Body;
/// # type Subject = CacheableHttpResponse<Empty<Bytes>>;
/// let predicate = StatusCode::new(http::StatusCode::OK)
///     .body(BodyOperation::Plain(PlainOperation::Contains("success".into())));
/// # let _: &Body<StatusCode<Neutral<Subject>>> = &predicate;
/// ```
#[derive(Debug)]
pub struct StatusCode<P> {
    operation: Operation,
    inner: P,
}

impl<S> StatusCode<Neutral<S>> {
    /// Creates a predicate matching a specific status code.
    pub fn new(status_code: http::StatusCode) -> Self {
        Self {
            operation: Operation::Eq(status_code),
            inner: Neutral::new(),
        }
    }
}

impl<P> StatusCode<P> {
    /// Creates a predicate matching any of the specified status codes.
    ///
    /// Returns [`Cacheable`](hitbox::predicate::PredicateResult::Cacheable) when
    /// the response status code is in the provided list.
    ///
    /// Use this for caching multiple specific status codes (e.g., 200 and 304).
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox::Neutral;
    /// use hitbox_http::predicates::response::StatusCode;
    ///
    /// # use bytes::Bytes;
    /// # use http_body_util::Empty;
    /// # use hitbox_http::CacheableHttpResponse;
    /// # type Subject = CacheableHttpResponse<Empty<Bytes>>;
    /// // Cache 200 OK and 304 Not Modified responses
    /// let predicate = StatusCode::new_in(
    ///     Neutral::new(),
    ///     vec![http::StatusCode::OK, http::StatusCode::NOT_MODIFIED],
    /// );
    /// # let _: &StatusCode<Neutral<Subject>> = &predicate;
    /// ```
    pub fn new_in(inner: P, codes: Vec<http::StatusCode>) -> Self {
        Self {
            operation: Operation::In(codes),
            inner,
        }
    }

    /// Creates a predicate matching status codes within a range (inclusive).
    ///
    /// Returns [`Cacheable`](hitbox::predicate::PredicateResult::Cacheable) when
    /// the response status code is between `start` and `end` (inclusive).
    ///
    /// Use this for custom status code ranges not covered by [`StatusClass`].
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox::Neutral;
    /// use hitbox_http::predicates::response::StatusCode;
    ///
    /// # use bytes::Bytes;
    /// # use http_body_util::Empty;
    /// # use hitbox_http::CacheableHttpResponse;
    /// # type Subject = CacheableHttpResponse<Empty<Bytes>>;
    /// // Cache responses with status codes 200-299 and 304
    /// let predicate = StatusCode::new_range(
    ///     Neutral::new(),
    ///     http::StatusCode::OK,
    ///     http::StatusCode::from_u16(299).unwrap(),
    /// );
    /// # let _: &StatusCode<Neutral<Subject>> = &predicate;
    /// ```
    pub fn new_range(inner: P, start: http::StatusCode, end: http::StatusCode) -> Self {
        Self {
            operation: Operation::Range(start, end),
            inner,
        }
    }

    /// Creates a predicate matching all status codes in a class.
    ///
    /// Returns [`Cacheable`](hitbox::predicate::PredicateResult::Cacheable) when
    /// the response status code belongs to the specified class (e.g., all 2xx).
    ///
    /// Use this for broad caching rules like "cache all successful responses".
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox::Neutral;
    /// use hitbox_http::predicates::response::{StatusCode, StatusClass};
    ///
    /// # use bytes::Bytes;
    /// # use http_body_util::Empty;
    /// # use hitbox_http::CacheableHttpResponse;
    /// # type Subject = CacheableHttpResponse<Empty<Bytes>>;
    /// // Cache all successful (2xx) responses
    /// let predicate = StatusCode::new_class(Neutral::new(), StatusClass::Success);
    /// # let _: &StatusCode<Neutral<Subject>> = &predicate;
    /// ```
    pub fn new_class(inner: P, class: StatusClass) -> Self {
        Self {
            operation: Operation::Class(class),
            inner,
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
    /// Matches an exact status code.
    fn status_code(self, status_code: http::StatusCode) -> StatusCode<Self>;
    /// Matches any of the specified status codes.
    fn status_code_in(self, codes: Vec<http::StatusCode>) -> StatusCode<Self>;
    /// Matches status codes within a range (inclusive).
    fn status_code_range(self, start: http::StatusCode, end: http::StatusCode) -> StatusCode<Self>;
    /// Matches all status codes in a class (e.g., all 2xx).
    fn status_code_class(self, class: StatusClass) -> StatusCode<Self>;
}

impl<P> StatusCodePredicate for P
where
    P: Predicate,
{
    fn status_code(self, status_code: http::StatusCode) -> StatusCode<Self> {
        StatusCode {
            operation: Operation::Eq(status_code),
            inner: self,
        }
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
