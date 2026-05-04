//! Request-side tag extractor configuration.
//!
//! [`RequestTagExtractor`] mirrors [`crate::extractors::Extractor`] and adds
//! the [`Static`](super::Static) variant. Every key extractor variant
//! (Path, Method, Query, Body, Header, Version) is reused here as a tag
//! extractor via the [`TagAdapter`](hitbox_core::tag::TagAdapter)
//! compatibility layer — each `KeyPart` produced by the underlying
//! extractor becomes a [`CacheTag`](hitbox_core::tag::CacheTag) of the form
//! `"key=value"` (or just `"key"` when the value is `None`).
//!
//! ```yaml
//! - Static:
//!     user: "42"
//! - Path: "/v1/authors/{author_id}/books/{book_id}"
//! - Method:
//! ```
//!
//! When multiple variants are supplied, the resulting tag extractors are
//! combined via [`ChainTagExtractor`](hitbox_core::tag::ChainTagExtractor)
//! at runtime — tags from each entry are concatenated in order.

use std::fmt::Debug;

use hitbox::config::{BoxExtractor, BoxTagExtractor};
use hitbox_core::tag::{ChainTagExtractor, ExtractorExt, NeutralTagExtractor, TagAdapter};
use hitbox_core::StaticExtractor;
use hitbox_http::CacheableHttpRequest;
use hitbox_http::extractors::NeutralExtractor;
use serde::{Deserialize, Serialize};

use super::Static;
use crate::error::ConfigError;
use crate::extractors::{
    body::BodyOperation, header::HeaderOperation, method::Method, path::Path,
    query::QueryOperation, version::Version, Extractor,
};

/// Request-side tag extractor configuration.
///
/// `Static` emits literal / `key=value` tags (see [`Static`]). All other
/// variants reuse a key [`Extractor`] and wrap it with [`TagAdapter`] —
/// the same compatibility layer exposed by [`ExtractorExt::as_tag`] in
/// `hitbox-core`.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum RequestTagExtractor {
    /// Subject-agnostic literal / `key=value` tags.
    Static(Static),
    /// Path key extractor reused as a tag extractor.
    Path(Path),
    /// HTTP method extractor reused as a tag extractor.
    Method(Method),
    /// Query parameter extractor reused as a tag extractor.
    Query(QueryOperation),
    /// Request body extractor reused as a tag extractor.
    Body(BodyOperation),
    /// Request header extractor reused as a tag extractor.
    Header(HeaderOperation),
    /// HTTP version extractor reused as a tag extractor.
    Version(Version),
}

impl RequestTagExtractor {
    /// Convert this configuration variant into a single boxed runtime tag
    /// extractor.
    fn into_tag_extractor<ReqBody>(
        self,
    ) -> Result<BoxTagExtractor<CacheableHttpRequest<ReqBody>>, ConfigError>
    where
        ReqBody: hyper::body::Body + Send + Debug + 'static,
        ReqBody::Error: Debug + Send,
        ReqBody::Data: Send,
    {
        match self {
            RequestTagExtractor::Static(s) => Ok(Box::new(
                StaticExtractor::<CacheableHttpRequest<ReqBody>>::new(s.into_key_parts()).as_tag(),
            )),
            RequestTagExtractor::Path(p) => wrap_extractor::<ReqBody>(Extractor::Path(p)),
            RequestTagExtractor::Method(m) => wrap_extractor::<ReqBody>(Extractor::Method(m)),
            RequestTagExtractor::Query(q) => wrap_extractor::<ReqBody>(Extractor::Query(q)),
            RequestTagExtractor::Body(b) => wrap_extractor::<ReqBody>(Extractor::Body(b)),
            RequestTagExtractor::Header(h) => wrap_extractor::<ReqBody>(Extractor::Header(h)),
            RequestTagExtractor::Version(v) => wrap_extractor::<ReqBody>(Extractor::Version(v)),
        }
    }
}

/// Build a single-extractor [`TagAdapter`] from one [`Extractor`] variant.
///
/// The extractor is chained onto a fresh [`NeutralExtractor`] base; the
/// resulting boxed key extractor is wrapped with [`TagAdapter`], turning
/// every [`KeyPart`](hitbox_core::KeyPart) it produces into a
/// [`CacheTag`](hitbox_core::tag::CacheTag) of the form `"key=value"`.
fn wrap_extractor<ReqBody>(
    ext: Extractor,
) -> Result<BoxTagExtractor<CacheableHttpRequest<ReqBody>>, ConfigError>
where
    ReqBody: hyper::body::Body + Send + Debug + 'static,
    ReqBody::Error: Debug + Send,
    ReqBody::Data: Send,
{
    let neutral: BoxExtractor<CacheableHttpRequest<ReqBody>> =
        Box::new(NeutralExtractor::<ReqBody>::new());
    let chained = ext.into_extractors(neutral)?;
    Ok(Box::new(TagAdapter::new(chained)))
}

/// Build a boxed runtime tag extractor for the request side from a list
/// of [`RequestTagExtractor`] config variants.
///
/// Multiple entries are composed via
/// [`ChainTagExtractor`](hitbox_core::tag::ChainTagExtractor) at runtime —
/// tags from each entry are concatenated in order.
pub fn build_request_boxed<ReqBody>(
    configs: Vec<RequestTagExtractor>,
) -> Result<BoxTagExtractor<CacheableHttpRequest<ReqBody>>, ConfigError>
where
    ReqBody: hyper::body::Body + Send + Debug + 'static,
    ReqBody::Error: Debug + Send,
    ReqBody::Data: Send,
{
    let mut tag_extractors: Vec<BoxTagExtractor<CacheableHttpRequest<ReqBody>>> =
        Vec::with_capacity(configs.len());
    for cfg in configs {
        tag_extractors.push(cfg.into_tag_extractor::<ReqBody>()?);
    }
    Ok(match tag_extractors.len() {
        0 => Box::new(NeutralTagExtractor::<CacheableHttpRequest<ReqBody>>::default()),
        1 => tag_extractors.into_iter().next().unwrap(),
        _ => Box::new(ChainTagExtractor::new(tag_extractors)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::path::Path;
    use indexmap::IndexMap;

    #[test]
    fn request_static_round_trips_via_yaml() {
        let yaml = r#"
            Static:
              user: "42"
        "#;
        let parsed: RequestTagExtractor = serde_saphyr::from_str(yaml).unwrap();
        let mut map = IndexMap::new();
        map.insert("user".into(), "42".into());
        assert_eq!(parsed, RequestTagExtractor::Static(Static::Map(map)));
    }

    #[test]
    fn request_path_round_trips_via_yaml() {
        let yaml = r#"Path: "/v1/authors/{author_id}/books/{book_id}""#;
        let parsed: RequestTagExtractor = serde_saphyr::from_str(yaml).unwrap();
        assert_eq!(
            parsed,
            RequestTagExtractor::Path(Path::new("/v1/authors/{author_id}/books/{book_id}"))
        );
    }

    #[test]
    fn request_list_of_mixed_variants_round_trips() {
        let yaml = r#"
            - Static:
                user: "42"
            - Path: "/v1/authors/{author_id}"
            - Method: {}
        "#;
        let parsed: Vec<RequestTagExtractor> = serde_saphyr::from_str(yaml).unwrap();
        assert_eq!(parsed.len(), 3);
        assert!(matches!(parsed[0], RequestTagExtractor::Static(_)));
        assert!(matches!(parsed[1], RequestTagExtractor::Path(_)));
        assert!(matches!(parsed[2], RequestTagExtractor::Method(_)));
    }
}
