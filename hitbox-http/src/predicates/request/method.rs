use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::Neutral;
use hitbox::predicate::{Predicate, PredicateResult};

/// Matching operations for HTTP methods.
#[derive(Debug)]
pub enum Operation {
    /// Match a single HTTP method.
    Eq(http::Method),
    /// Match any of the specified HTTP methods.
    In(Vec<http::Method>),
}

/// A predicate that matches requests by HTTP method.
///
/// # Type Parameters
///
/// * `P` - The inner predicate to chain with. Use [`Method::new`] to start
///   a new predicate chain (uses [`Neutral`] internally), or use the
///   [`MethodPredicate`] extension trait to chain onto an existing predicate.
///
/// # Examples
///
/// Match only GET requests:
///
/// ```
/// use hitbox_http::predicates::request::Method;
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox::Neutral;
/// # use hitbox_http::CacheableHttpRequest;
/// # type Subject = CacheableHttpRequest<Empty<Bytes>>;
/// let predicate = Method::new(http::Method::GET).unwrap();
/// # let _: &Method<Neutral<Subject>> = &predicate;
/// ```
///
/// Match GET or HEAD requests (using the builder pattern):
///
/// ```
/// use hitbox::Neutral;
/// use hitbox_http::predicates::request::Method;
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox_http::CacheableHttpRequest;
/// # type Subject = CacheableHttpRequest<Empty<Bytes>>;
/// let predicate = Method::new_in(
///     Neutral::new(),
///     vec![http::Method::GET, http::Method::HEAD],
/// );
/// # let _: &Method<Neutral<Subject>> = &predicate;
/// ```
#[derive(Debug)]
pub struct Method<P> {
    operation: Operation,
    inner: P,
}

impl<S> Method<Neutral<S>> {
    /// Creates a predicate matching requests with the specified HTTP method.
    ///
    /// Returns [`Cacheable`](hitbox::predicate::PredicateResult::Cacheable) when
    /// the request method matches, [`NonCacheable`](hitbox::predicate::PredicateResult::NonCacheable) otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if `method` cannot be converted to [`http::Method`].
    /// When passing `http::Method` directly, this is infallible.
    /// When passing a string, returns [`http::method::InvalidMethod`] if the
    /// string is not a valid HTTP method.
    pub fn new<E, T>(method: T) -> Result<Self, E>
    where
        T: TryInto<http::Method, Error = E>,
    {
        Ok(Method {
            operation: Operation::Eq(method.try_into()?),
            inner: Neutral::new(),
        })
    }
}

impl<P> Method<P> {
    /// Creates a predicate matching requests with any of the specified HTTP methods.
    ///
    /// Returns [`Cacheable`](hitbox::predicate::PredicateResult::Cacheable) when
    /// the request method is in the provided list, [`NonCacheable`](hitbox::predicate::PredicateResult::NonCacheable) otherwise.
    ///
    /// Use this for caching strategies that apply to multiple methods (e.g., GET and HEAD).
    pub fn new_in(inner: P, methods: Vec<http::Method>) -> Self {
        Method {
            operation: Operation::In(methods),
            inner,
        }
    }
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
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
{
    type Subject = P::Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(request).await {
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
