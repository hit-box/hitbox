use hitbox::{
    CacheablePolicyData, RequestCachePolicy,
    predicate::{Predicate, PredicateResult},
    {CachePolicy, CacheableRequest, Extractor},
};
use http::{Request, request::Parts};
use hyper::body::Body as HttpBody;

use crate::CacheableSubject;
use crate::body::BufferedBody;
use crate::predicates::header::HasHeaders;
use crate::predicates::version::HasVersion;

/// Wraps an HTTP request for cache policy evaluation.
///
/// This type holds the request metadata ([`Parts`]) and a [`BufferedBody`] that
/// allows predicates and extractors to inspect the request without fully consuming
/// the body stream.
///
/// # Type Parameters
///
/// * `ReqBody` - The HTTP request body type. Must implement [`hyper::body::Body`]
///   with `Send` bounds. Common concrete types:
///   - [`Empty<Bytes>`](http_body_util::Empty) - No body (GET requests)
///   - [`Full<Bytes>`](http_body_util::Full) - Complete body in memory
///   - `BoxBody<Bytes, E>` - Type-erased body for dynamic dispatch
///
/// # Examples
///
/// ```
/// use bytes::Bytes;
/// use http::Request;
/// use http_body_util::Empty;
/// use hitbox_http::{BufferedBody, CacheableHttpRequest};
///
/// let request = Request::builder()
///     .method("GET")
///     .uri("/users/42")
///     .header("Authorization", "Bearer token")
///     .body(BufferedBody::Passthrough(Empty::<Bytes>::new()))
///     .unwrap();
///
/// let cacheable = CacheableHttpRequest::from_request(request);
/// ```
///
/// # Extracting Cache Keys
///
/// Use with extractors to generate cache key parts:
///
/// ```no_run
/// use hitbox::Extractor;
/// use hitbox_http::CacheableHttpRequest;
/// use hitbox_http::extractors::{Method, path::PathExtractor};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// # use hitbox_http::extractors::{NeutralExtractor, Path};
/// async fn example(cacheable: CacheableHttpRequest<Empty<Bytes>>) {
///     let extractor = Method::new().path("/users/{user_id}");
///     # let _: &Path<Method<NeutralExtractor<Empty<Bytes>>>> = &extractor;
///     let key_parts = extractor.get(cacheable).await;
/// }
/// ```
#[derive(Debug)]
pub struct CacheableHttpRequest<ReqBody>
where
    ReqBody: HttpBody,
{
    parts: Parts,
    body: BufferedBody<ReqBody>,
}

impl<ReqBody> CacheableHttpRequest<ReqBody>
where
    ReqBody: HttpBody,
{
    /// Creates a cacheable request from an HTTP request with a buffered body.
    ///
    /// The request body must already be wrapped in a [`BufferedBody`]. Use
    /// [`BufferedBody::Passthrough`] for requests that haven't been inspected yet.
    pub fn from_request(request: Request<BufferedBody<ReqBody>>) -> Self {
        let (parts, body) = request.into_parts();
        Self { parts, body }
    }

    /// Converts back into a standard HTTP request.
    ///
    /// Use this after cache policy evaluation to continue processing the request.
    pub fn into_request(self) -> Request<BufferedBody<ReqBody>> {
        Request::from_parts(self.parts, self.body)
    }

    /// Returns a reference to the request metadata.
    ///
    /// The [`Parts`] contain the method, URI, version, headers, and extensions.
    pub fn parts(&self) -> &Parts {
        &self.parts
    }

    /// Decomposes into metadata and body.
    ///
    /// This is equivalent to [`CacheableSubject::into_parts`].
    pub fn into_parts(self) -> (Parts, BufferedBody<ReqBody>) {
        (self.parts, self.body)
    }
}

impl<ReqBody> CacheableSubject for CacheableHttpRequest<ReqBody>
where
    ReqBody: HttpBody,
{
    type Body = ReqBody;
    type Parts = Parts;

    fn into_parts(self) -> (Self::Parts, BufferedBody<Self::Body>) {
        (self.parts, self.body)
    }

    fn from_parts(parts: Self::Parts, body: BufferedBody<Self::Body>) -> Self {
        Self { parts, body }
    }
}

impl<ReqBody> HasHeaders for CacheableHttpRequest<ReqBody>
where
    ReqBody: HttpBody,
{
    fn headers(&self) -> &http::HeaderMap {
        &self.parts.headers
    }
}

impl<ReqBody> HasVersion for CacheableHttpRequest<ReqBody>
where
    ReqBody: HttpBody,
{
    fn http_version(&self) -> http::Version {
        self.parts.version
    }
}

impl<ReqBody> CacheableRequest for CacheableHttpRequest<ReqBody>
where
    ReqBody: HttpBody + Send + 'static,
    ReqBody::Error: Send,
{
    async fn cache_policy<P, E>(self, predicates: P, extractors: E) -> RequestCachePolicy<Self>
    where
        P: Predicate<Subject = Self> + Send + Sync,
        E: Extractor<Subject = Self> + Send + Sync,
    {
        let (request, key) = extractors.get(self).await.into_cache_key();

        match predicates.check(request).await {
            PredicateResult::Cacheable(request) => {
                CachePolicy::Cacheable(CacheablePolicyData { key, request })
            }
            PredicateResult::NonCacheable(request) => CachePolicy::NonCacheable(request),
        }
    }
}
