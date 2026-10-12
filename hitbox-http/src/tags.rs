//! Response-side cache tag extractors.
//!
//! Request-side tags reuse key [`Extractor`](hitbox::Extractor)s via the
//! [`TagAdapter`](hitbox::tag::TagAdapter) compatibility layer. The response
//! side has no key extractors — a response never contributes to the cache key
//! — so these are direct [`TagExtractor`](hitbox::tag::TagExtractor) implementations over
//! [`CacheableHttpResponse`].
//!
//! Header reading itself is **not** duplicated: it lives on
//! [`HeaderConfig::select`](crate::extractors::header::HeaderConfig::select),
//! shared with the request key [`Header`](crate::extractors::header::Header)
//! extractor. The response side adds only the [`KeyPart`](hitbox::KeyPart) →
//! [`CacheTag`](hitbox::tag::CacheTag)
//! projection (the same `"name=value"` conversion the request `TagAdapter`
//! uses).

use std::marker::PhantomData;

use async_trait::async_trait;
use hitbox::tag::{CacheTag, TagExtractor};
use hyper::body::Body as HttpBody;

use crate::CacheableHttpResponse;
use crate::extractors::header::HeaderConfig;
use crate::predicates::header::HasHeaders;

/// Emits cache tags from a response header, reusing the request-side
/// [`HeaderConfig`] selection machinery (exact / prefix name selection, regex
/// value extraction, transforms).
///
/// The canonical use case is `ETag` — tagging a cached entry with the entity
/// version the upstream returned, so that version can later be invalidated. A
/// regex value extractor can strip the surrounding quotes, e.g. turning the
/// header `ETag: "abc123"` into the tag `etag=abc123`.
///
/// Each selected header becomes a `"<name>=<value>"` tag via the same
/// [`KeyPart`](hitbox::KeyPart) → [`CacheTag`] conversion the request-side
/// `TagAdapter` uses. Headers that are absent (no value) produce no tag.
///
/// The `ResBody` parameter only fixes the response body type of the
/// [`Subject`](TagExtractor::Subject); the extractor holds no body.
/// `PhantomData<fn() -> ResBody>` keeps it `Send + Sync` regardless of
/// `ResBody`.
pub struct ResponseHeaderTagExtractor<ResBody> {
    config: HeaderConfig,
    _phantom: PhantomData<fn() -> ResBody>,
}

impl<ResBody> ResponseHeaderTagExtractor<ResBody> {
    /// Create an extractor from a [`HeaderConfig`].
    ///
    /// Accepts a [`HeaderConfig`] or a string (exact header name, full value)
    /// directly, mirroring the request-side
    /// [`HeaderExtractor::header`](crate::extractors::header::HeaderExtractor::header).
    pub fn new(config: impl Into<HeaderConfig>) -> Self {
        Self {
            config: config.into(),
            _phantom: PhantomData,
        }
    }
}

impl<ResBody> std::fmt::Debug for ResponseHeaderTagExtractor<ResBody> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResponseHeaderTagExtractor")
            .field("config", &self.config)
            .finish()
    }
}

#[async_trait]
impl<ResBody> TagExtractor for ResponseHeaderTagExtractor<ResBody>
where
    ResBody: HttpBody + Send + 'static,
    ResBody::Error: Send,
    ResBody::Data: Send,
{
    type Subject = CacheableHttpResponse<ResBody>;

    async fn extract_tags(&self, subject: Self::Subject) -> (Self::Subject, Vec<CacheTag>) {
        // Reuse the shared header selection, then project the parts that
        // actually carried a value into `"name=value"` tags. Absent headers
        // (value `None`) yield no tag.
        let tags = self
            .config
            .select(subject.headers())
            .iter()
            .filter(|part| part.value().is_some())
            .map(CacheTag::from)
            .collect();
        (subject, tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BufferedBody;
    use crate::extractors::header::{NameSelector, ValueExtractor};
    use bytes::Bytes;
    use http::Response;
    use http_body_util::Full;
    use regex::Regex;

    fn response_with(headers: &[(&str, &str)]) -> CacheableHttpResponse<Full<Bytes>> {
        let mut builder = Response::builder().status(200);
        for (k, v) in headers {
            builder = builder.header(*k, *v);
        }
        let resp = builder
            .body(BufferedBody::Passthrough(Full::new(Bytes::from_static(
                b"body",
            ))))
            .unwrap();
        CacheableHttpResponse::from_response(resp)
    }

    #[tokio::test]
    async fn emits_key_value_tag_from_present_header() {
        // Tag key is the configured header name; the value comes from the
        // response header (header lookup itself is case-insensitive).
        let ext = ResponseHeaderTagExtractor::<Full<Bytes>>::new("etag");
        let resp = response_with(&[("etag", "abc123")]);
        let (_subject, tags) = ext.extract_tags(resp).await;
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].as_str(), "etag=abc123");
    }

    #[tokio::test]
    async fn no_tag_when_header_absent() {
        let ext = ResponseHeaderTagExtractor::<Full<Bytes>>::new("etag");
        let resp = response_with(&[("server", "hitbox-test")]);
        let (_subject, tags) = ext.extract_tags(resp).await;
        assert!(tags.is_empty());
    }

    #[tokio::test]
    async fn regex_value_extractor_strips_etag_quotes() {
        // The shared HeaderConfig machinery gives the response side regex value
        // extraction for free — here, stripping the quotes around an ETag.
        let config = HeaderConfig::name(NameSelector::exact("etag"))
            .value(ValueExtractor::Regex(Regex::new("\"(.+)\"").unwrap()));
        let ext = ResponseHeaderTagExtractor::<Full<Bytes>>::new(config);
        let resp = response_with(&[("etag", "\"abc123\"")]);
        let (_subject, tags) = ext.extract_tags(resp).await;
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].as_str(), "etag=abc123");
    }
}
