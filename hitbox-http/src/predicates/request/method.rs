use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::EvalContext;
use hitbox::predicate::{Predicate, PredicateResult};

/// Matching operations for HTTP methods.
#[derive(Debug)]
pub enum Operation {
    /// Match a single HTTP method.
    Eq(http::Method),
    /// Match any of the specified HTTP methods.
    In(Vec<http::Method>),
}

impl Operation {
    /// Match a specific HTTP method.
    pub fn eq(method: http::Method) -> Self {
        Operation::Eq(method)
    }

    /// Match any of the specified HTTP methods.
    pub fn any(methods: Vec<http::Method>) -> Self {
        Operation::In(methods)
    }
}

impl From<http::Method> for Operation {
    fn from(method: http::Method) -> Self {
        Operation::Eq(method)
    }
}

/// A predicate that matches requests by HTTP method.
///
/// # Type Parameters
///
/// * `P` - The inner predicate to chain with. Use [`request::predicate()`](super::predicate)
///   to start a new chain, then call `.method(...)`.
///
/// # Examples
///
/// Match only GET requests:
///
/// ```
/// use hitbox_http::predicates::request::{self, MethodPredicate};
/// use hitbox_http::predicates::request::method::Operation;
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// let predicate = request::predicate::<Empty<Bytes>>()
///     .method(Operation::eq(http::Method::GET));
/// ```
///
/// Match GET or HEAD requests:
///
/// ```
/// use hitbox_http::predicates::request::{self, MethodPredicate};
/// use hitbox_http::predicates::request::method::Operation;
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// let predicate = request::predicate::<Empty<Bytes>>()
///     .method(Operation::any(vec![http::Method::GET, http::Method::HEAD]));
/// ```
#[derive(Debug)]
pub struct Method<P> {
    pub(crate) operation: Operation,
    pub(crate) inner: P,
}

/// Extension trait for adding method matching to a predicate chain.
///
/// # For Callers
///
/// Chain this to match requests by their HTTP method. Use with specific
/// methods like `http::Method::GET` or `http::Method::POST`.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Predicate`]
/// types. You don't need to implement it manually.
pub trait MethodPredicate: Sized {
    /// Adds an HTTP method match to this predicate chain.
    ///
    /// Accepts an [`Operation`] or an [`http::Method`] directly.
    fn method(self, operation: impl Into<Operation>) -> Method<Self>;
}

impl<P> MethodPredicate for P
where
    P: Predicate,
{
    fn method(self, operation: impl Into<Operation>) -> Method<Self> {
        Method {
            operation: operation.into(),
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for Method<P>
where
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
{
    type Subject = P::Subject;

    async fn check(
        &self,
        request: Self::Subject,
        ctx: &EvalContext,
    ) -> PredicateResult<Self::Subject> {
        match self.inner.check(request, ctx).await {
            PredicateResult::Cacheable(request) => {
                let is_cacheable = match &self.operation {
                    Operation::Eq(method) => *method == request.parts().method,
                    Operation::In(methods) => methods.contains(&request.parts().method),
                };
                if is_cacheable {
                    PredicateResult::Cacheable(request)
                } else {
                    PredicateResult::NonCacheable(request)
                }
            }
            PredicateResult::NonCacheable(request) => PredicateResult::NonCacheable(request),
        }
    }
}
