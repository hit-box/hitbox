//! Query parameter matching predicate.
//!
//! Provides [`Query`] predicate and [`Operation`] for matching URL query parameters.

use crate::CacheableHttpRequest;
use async_trait::async_trait;
use hitbox::Neutral;
use hitbox::predicate::{Predicate, PredicateResult};

/// Operations for matching query parameters.
///
/// # Variants
///
/// - [`Eq`](Self::Eq) — Parameter must equal a specific value
/// - [`Exist`](Self::Exist) — Parameter must be present
/// - [`In`](Self::In) — Parameter value must be one of the specified values
#[derive(Debug)]
pub enum Operation {
    /// Match if the parameter equals the value. Format: `(name, expected_value)`.
    Eq(String, String),
    /// Match if the parameter exists (regardless of value).
    Exist(String),
    /// Match if the parameter value is one of these values. Format: `(name, allowed_values)`.
    In(String, Vec<String>),
}

impl Operation {
    /// Creates an operation matching a query parameter with an exact value.
    pub fn eq(name: impl Into<String>, value: impl Into<String>) -> Self {
        Operation::Eq(name.into(), value.into())
    }

    /// Creates an operation matching the existence of a query parameter.
    pub fn exist(name: impl Into<String>) -> Self {
        Operation::Exist(name.into())
    }

    /// Creates an operation matching a query parameter against multiple values.
    pub fn any(name: impl Into<String>, values: Vec<String>) -> Self {
        Operation::In(name.into(), values)
    }
}

impl From<&str> for Operation {
    /// Shorthand for `Operation::exist(name)`.
    fn from(name: &str) -> Self {
        Operation::exist(name)
    }
}

impl From<(&str, &str)> for Operation {
    /// Shorthand for `Operation::eq(name, value)`.
    fn from((name, value): (&str, &str)) -> Self {
        Operation::eq(name, value)
    }
}

/// A predicate that matches requests by query parameters.
///
/// Returns [`Cacheable`](PredicateResult::Cacheable) when the query parameter
/// satisfies the operation, [`NonCacheable`](PredicateResult::NonCacheable) otherwise.
///
/// # Type Parameters
///
/// * `P` - The inner predicate to chain with. Use [`Query::new`] to start
///   a new predicate chain (uses [`Neutral`] internally), or use the
///   [`QueryPredicate`] extension trait to chain onto an existing predicate.
///
/// # Examples
///
/// ```
/// use hitbox_http::predicates::request::query::{Query, Operation};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox::Neutral;
/// # use hitbox_http::CacheableHttpRequest;
/// # type Subject = CacheableHttpRequest<Empty<Bytes>>;
/// // Cache only when "format" query parameter is "json"
/// let predicate = Query::new(Operation::eq("format", "json"));
/// # let _: &Query<Neutral<Subject>> = &predicate;
/// ```
#[derive(Debug)]
pub struct Query<P> {
    pub(crate) operation: Operation,
    pub(crate) inner: P,
}

impl<S> Query<Neutral<S>> {
    /// Creates a standalone query predicate from an [`Operation`].
    ///
    /// For chaining, use the [`QueryPredicate`] extension trait directly.
    pub fn new(operation: Operation) -> Self {
        Self {
            operation,
            inner: Neutral::new(),
        }
    }
}

/// Extension trait for adding query parameter matching to a predicate chain.
///
/// # For Callers
///
/// Chain this to match requests by their URL query parameters. Use the
/// [`Operation`] enum to specify exact matches, existence checks, or
/// set membership.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Predicate`]
/// types. You don't need to implement it manually.
pub trait QueryPredicate: Sized {
    /// Adds a query parameter match to this predicate chain.
    ///
    /// Accepts an [`Operation`], a `&str` (existence check), or
    /// `(&str, &str)` (exact match) directly.
    fn query(self, operation: impl Into<Operation>) -> Query<Self>;
}

impl<P> QueryPredicate for P
where
    P: Predicate,
{
    fn query(self, operation: impl Into<Operation>) -> Query<Self> {
        Query {
            operation: operation.into(),
            inner: self,
        }
    }
}

#[async_trait]
impl<P, ReqBody> Predicate for Query<P>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = P::Subject;

    async fn check(&self, request: Self::Subject) -> PredicateResult<Self::Subject> {
        match self.inner.check(request).await {
            PredicateResult::Cacheable(request) => {
                let is_cacheable = match request.parts().uri.query().and_then(crate::query::parse) {
                    Some(query_map) => match &self.operation {
                        Operation::Eq(name, value) => query_map
                            .get(name)
                            .map(|v| v.contains(value))
                            .unwrap_or_default(),
                        Operation::Exist(name) => {
                            query_map.get(name).map(|_| true).unwrap_or_default()
                        }
                        Operation::In(name, values) => query_map
                            .get(name)
                            .and_then(|value| values.iter().find(|v| value.contains(v)))
                            .map(|_| true)
                            .unwrap_or_default(),
                    },
                    None => false,
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
