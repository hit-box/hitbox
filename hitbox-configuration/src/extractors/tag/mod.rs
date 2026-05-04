//! Configuration types for cache tag extractors.
//!
//! Tag extraction config is split per side:
//!
//! - [`request::RequestTagExtractor`] — request-side. Supports [`Static`]
//!   (subject-agnostic literal / key=value tags) plus every variant of
//!   [`crate::extractors::Extractor`] (Path, Method, Query, Body, Header,
//!   Version) — those reuse the
//!   [`TagAdapter`](hitbox_core::tag::TagAdapter) compatibility layer to
//!   turn a key extractor into a tag extractor.
//! - [`ResponseTagExtractor`] — response-side. Currently only `Static`.
//!   Dynamic response variants (e.g. extracting an ETag header from the
//!   response, or fields from the response body) are blocked on a
//!   response-shape extractor enum that does not yet exist; once it lands,
//!   mirrored variants will be added here.
//!
//! Both sides share the [`Static`] form, which emits a fixed list of tags.
//! Three YAML shapes are supported (no overlap, no string parsing — each
//! shape has a single, unambiguous semantic):
//!
//! ```yaml
//! # Strings → bare-key tags (literal tag names with no `=`).
//! - Static: "endpoint:users"
//! - Static: ["v1", "deprecated"]
//!
//! # Mapping → "key=value" tags (insertion order preserved).
//! - Static:
//!     user: "42"
//!     region: "eu"
//! ```

pub mod request;

use hitbox::config::BoxTagExtractor;
use hitbox_core::tag::ExtractorExt;
use hitbox_core::{KeyPart, StaticExtractor};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub use request::RequestTagExtractor;

/// Static tag form — three YAML shapes, each with a single semantic.
///
/// Strings are **literal tag names** (bare-key
/// [`CacheTag`](hitbox_core::tag::CacheTag)s with no `=`). Mappings are
/// **`"key=value"` tags**. The forms do not overlap — there's no string
/// parsing, each shape unambiguously describes what it produces.
///
/// ```yaml
/// # Single literal tag.
/// Static: "endpoint:users"
///
/// # List of literal tags.
/// Static:
///   - "v1"
///   - "deprecated"
///
/// # Native mapping → "key=value" tags, insertion order preserved.
/// Static:
///   user: "42"
///   region: "eu"
/// ```
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum Static {
    /// Single literal tag (bare key, no value).
    Single(String),
    /// List of literal tags (each a bare key, no value).
    List(Vec<String>),
    /// Mapping of `key → value` pairs, emitted as `"key=value"` tags.
    Map(IndexMap<String, String>),
}

impl Static {
    /// Convert into the [`KeyPart`] sequence consumed by
    /// [`StaticExtractor`] (and via [`ExtractorExt::as_tag`], by
    /// [`TagAdapter`](hitbox_core::tag::TagAdapter)).
    pub(crate) fn into_key_parts(self) -> Vec<KeyPart> {
        match self {
            Static::Single(s) => vec![KeyPart::new(s, None::<&str>)],
            Static::List(items) => items
                .into_iter()
                .map(|s| KeyPart::new(s, None::<&str>))
                .collect(),
            Static::Map(map) => map
                .into_iter()
                .map(|(k, v)| KeyPart::new(k, Some(v)))
                .collect(),
        }
    }
}

/// Configuration enum for response-side tag extractors.
///
/// Currently only `Static` is supported. Once a response-shape extractor
/// enum exists, its variants will be mirrored here for the same
/// [`TagAdapter`](hitbox_core::tag::TagAdapter)-based compatibility layer
/// already used for request-side tags.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum ResponseTagExtractor {
    /// Emit a fixed list of tags. See [`Static`] for the supported shapes.
    Static(Static),
}

impl ResponseTagExtractor {
    /// Convert this configuration variant into a runtime tag extractor.
    pub fn into_tag_extractor<S>(self) -> BoxTagExtractor<S>
    where
        S: Send + 'static,
    {
        match self {
            ResponseTagExtractor::Static(s) => {
                Box::new(StaticExtractor::<S>::new(s.into_key_parts()).as_tag())
            }
        }
    }
}

/// Build a boxed runtime tag extractor for the response side from a list
/// of [`ResponseTagExtractor`] config variants.
pub fn build_response_boxed<S>(configs: Vec<ResponseTagExtractor>) -> BoxTagExtractor<S>
where
    S: Send + 'static,
{
    let mut all_parts: Vec<KeyPart> = Vec::new();
    for cfg in configs {
        match cfg {
            ResponseTagExtractor::Static(s) => {
                all_parts.extend(s.into_key_parts());
            }
        }
    }
    Box::new(StaticExtractor::<S>::new(all_parts).as_tag())
}

/// Grouping of request- and response-side tag extractor configs.
///
/// Mirrors the YAML shape:
///
/// ```yaml
/// tags:
///   request:
///     - Static:
///         user: "42"
///     - Path: "/v1/authors/{author_id}/books/{book_id}"
///   response:
///     - Static:
///         etag: "v1"
/// ```
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct TagsConfig {
    /// Request-side tag extractors. Empty = neutral (no request tags).
    #[serde(default)]
    pub request: Vec<RequestTagExtractor>,
    /// Response-side tag extractors. Empty = neutral (no response tags).
    #[serde(default)]
    pub response: Vec<ResponseTagExtractor>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hitbox_core::tag::TagExtractor as TagExtractorTrait;

    #[tokio::test]
    async fn static_single_emits_one_literal_tag() {
        let cfg = ResponseTagExtractor::Static(Static::Single("endpoint:users".into()));
        let ext: BoxTagExtractor<()> = cfg.into_tag_extractor();
        let (_subject, tags) = ext.extract_tags(()).await;
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].as_str(), "endpoint:users");
    }

    #[tokio::test]
    async fn static_list_emits_literal_tags() {
        let cfg =
            ResponseTagExtractor::Static(Static::List(vec!["v1".into(), "deprecated".into()]));
        let ext: BoxTagExtractor<()> = cfg.into_tag_extractor();
        let (_subject, tags) = ext.extract_tags(()).await;
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].as_str(), "v1");
        assert_eq!(tags[1].as_str(), "deprecated");
    }

    #[tokio::test]
    async fn static_map_emits_key_value_tags_in_insertion_order() {
        let mut map = IndexMap::new();
        map.insert("user".into(), "42".into());
        map.insert("region".into(), "eu".into());
        let cfg = ResponseTagExtractor::Static(Static::Map(map));
        let ext: BoxTagExtractor<()> = cfg.into_tag_extractor();
        let (_subject, tags) = ext.extract_tags(()).await;
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].as_str(), "user=42");
        assert_eq!(tags[1].as_str(), "region=eu");
    }

    #[test]
    fn static_single_round_trips_via_yaml() {
        let yaml = r#"Static: "endpoint:users""#;
        let parsed: ResponseTagExtractor = serde_saphyr::from_str(yaml).unwrap();
        assert_eq!(
            parsed,
            ResponseTagExtractor::Static(Static::Single("endpoint:users".into()))
        );
    }

    #[test]
    fn static_list_round_trips_via_yaml() {
        let yaml = r#"
            Static:
              - "v1"
              - "deprecated"
        "#;
        let parsed: ResponseTagExtractor = serde_saphyr::from_str(yaml).unwrap();
        assert_eq!(
            parsed,
            ResponseTagExtractor::Static(Static::List(vec!["v1".into(), "deprecated".into()]))
        );
    }

    #[test]
    fn static_map_round_trips_via_yaml() {
        let yaml = r#"
            Static:
              user: "42"
              region: "eu"
        "#;
        let parsed: ResponseTagExtractor = serde_saphyr::from_str(yaml).unwrap();
        let mut map = IndexMap::new();
        map.insert("user".into(), "42".into());
        map.insert("region".into(), "eu".into());
        assert_eq!(parsed, ResponseTagExtractor::Static(Static::Map(map)));
    }

    #[test]
    fn tags_config_round_trips_with_both_sides() {
        let yaml = r#"
            request:
              - Static:
                  user: "42"
            response:
              - Static:
                  etag: "v1"
        "#;
        let parsed: TagsConfig = serde_saphyr::from_str(yaml).unwrap();
        assert_eq!(parsed.request.len(), 1);
        assert_eq!(parsed.response.len(), 1);
    }

    #[test]
    fn tags_config_omits_either_side() {
        let yaml = r#"
            response:
              - Static:
                  etag: "v1"
        "#;
        let parsed: TagsConfig = serde_saphyr::from_str(yaml).unwrap();
        assert!(parsed.request.is_empty());
        assert_eq!(parsed.response.len(), 1);
    }
}
