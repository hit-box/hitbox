use async_trait::async_trait;
use hitbox::EvalContext;
use hitbox::{Extractor, KeyPart, KeyParts};

use crate::CacheableHttpRequest;

/// Extracts the HTTP method as a cache key part.
///
/// Adds a key part with name `"method"` and the method as value (e.g., `"GET"`, `"POST"`).
/// Use this as the starting point for extractor chains.
///
/// # Type Parameters
///
/// * `E` - The inner extractor to chain with. Use [`extractors::extractor()`](super::extractor)
///   to start a new chain, then call `.method(MethodConfig::new())`.
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::{self, MethodConfig, MethodExtractor, PathExtractor};
/// use hitbox_http::extractors::query::QueryExtractor;
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox_http::extractors::{NeutralExtractor, Method, Path, query::Query};
/// let extractor = extractors::extractor::<Empty<Bytes>>()
///     .method(MethodConfig::new())
///     .path("/users/{user_id}")
///     .query("page");
/// # let _: &Query<Path<Method<NeutralExtractor<Empty<Bytes>>>>> = &extractor;
/// ```
///
/// # Key Parts Generated
///
/// Generates a single key part: `method={METHOD}` where `{METHOD}` is the
/// uppercase HTTP method name (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS, etc.).
#[derive(Debug)]
pub struct Method<E> {
    inner: E,
}

/// Configuration for the method extractor.
///
/// This is a marker type with no configuration options — the HTTP method
/// is always extracted as-is.
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::{self, MethodConfig, MethodExtractor};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// let extractor = extractors::extractor::<Empty<Bytes>>()
///     .method(MethodConfig::new());
/// ```
#[derive(Debug, Clone, Default)]
pub struct MethodConfig;

impl MethodConfig {
    /// Creates a new method extractor configuration.
    pub fn new() -> Self {
        MethodConfig
    }
}

/// Extension trait for adding method extraction to an extractor chain.
///
/// # For Callers
///
/// Chain this after [`extractors::extractor()`](super::extractor) or any other extractor to add the HTTP
/// method to your cache key. The method is added as a key part with name
/// `"method"` and value like `"GET"` or `"POST"`.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Extractor`]
/// types. You don't need to implement it manually.
pub trait MethodExtractor: Sized {
    /// Adds HTTP method extraction to the chain.
    fn method(self, config: MethodConfig) -> Method<Self>;
}

impl<E> MethodExtractor for E
where
    E: Extractor,
{
    fn method(self, _config: MethodConfig) -> Method<Self> {
        Method { inner: self }
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for Method<E>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject, ctx: &EvalContext) -> KeyParts<Self::Subject> {
        let method = subject.parts().method.to_string();
        let mut parts = self.inner.get(subject, ctx).await;
        parts.push(KeyPart::new("method", Some(method)));
        parts
    }
}
