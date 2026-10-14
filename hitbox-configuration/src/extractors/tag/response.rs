//! Response-side tag extractor configuration.
//!
//! [`ResponseTagExtractor`] mirrors the request side's
//! [`RequestTagExtractor`](super::RequestTagExtractor) where it can:
//!
//! - `Static` — subject-agnostic literal / `key=value` tags (see
//!   [`Static`]).
//! - `Header` — a `"<header>=<value>"` tag from a response header, reusing the
//!   request-side [`HeaderOperation`]
//!   config and the shared
//!   [`HeaderConfig::select`](hitbox_http::extractors::header::HeaderConfig::select)
//!   machinery (exact / prefix selection, regex value extraction, transforms).
//!
//! There is no `Path` / `Method` / etc. on the response side: those read
//! request data. Body-derived response tags are a future addition.
//!
//! When multiple entries are supplied they are combined via
//! [`ChainTagExtractor`] at runtime.

use std::fmt::Debug;

use hitbox::config::BoxTagExtractor;
use hitbox_core::StaticExtractor;
use hitbox_core::tag::{ChainTagExtractor, ExtractorExt, NeutralTagExtractor};
use hitbox_http::CacheableHttpResponse;
use hitbox_http::ResponseHeaderTagExtractor;
use serde::{Deserialize, Serialize};

use super::Static;
use crate::error::ConfigError;
use crate::extractors::header::HeaderOperation;

/// Response-side tag extractor configuration.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum ResponseTagExtractor {
    /// Subject-agnostic literal / `key=value` tags. See [`Static`].
    Static(Static),
    /// Emit a `"<header>=<value>"` tag from a response header.
    ///
    /// Accepts the same configuration as the request-side `Header` extractor
    /// (`Header: "etag"`, or an extended form with regex value extraction and
    /// transforms).
    Header(HeaderOperation),
}

impl ResponseTagExtractor {
    /// Convert this configuration variant into a single boxed runtime tag
    /// extractor over the cacheable HTTP response.
    fn into_tag_extractor<ResBody>(
        self,
    ) -> Result<BoxTagExtractor<CacheableHttpResponse<ResBody>>, ConfigError>
    where
        ResBody: hyper::body::Body + Send + 'static,
        ResBody::Error: Send,
        ResBody::Data: Send,
    {
        match self {
            ResponseTagExtractor::Static(s) => Ok(Box::new(
                StaticExtractor::<CacheableHttpResponse<ResBody>>::new(s.into_key_parts())
                    .into_tag_extractor(),
            )),
            ResponseTagExtractor::Header(op) => {
                let config = op.into_header_config()?;
                Ok(Box::new(ResponseHeaderTagExtractor::<ResBody>::new(config)))
            }
        }
    }
}

/// Build a boxed runtime tag extractor for the response side from a list of
/// [`ResponseTagExtractor`] config variants.
///
/// Multiple entries are composed via
/// [`ChainTagExtractor`] at runtime — tags
/// from each entry are concatenated in order.
pub fn build_response_boxed<ResBody>(
    configs: Vec<ResponseTagExtractor>,
) -> Result<BoxTagExtractor<CacheableHttpResponse<ResBody>>, ConfigError>
where
    ResBody: hyper::body::Body + Send + 'static,
    ResBody::Error: Send,
    ResBody::Data: Send,
{
    let mut tag_extractors: Vec<BoxTagExtractor<CacheableHttpResponse<ResBody>>> =
        Vec::with_capacity(configs.len());
    for cfg in configs {
        tag_extractors.push(cfg.into_tag_extractor::<ResBody>()?);
    }
    Ok(match tag_extractors.len() {
        0 => Box::new(NeutralTagExtractor::<CacheableHttpResponse<ResBody>>::default()),
        1 => tag_extractors.into_iter().next().unwrap(),
        _ => Box::new(ChainTagExtractor::new(tag_extractors)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use hitbox_core::tag::TagExtractor as TagExtractorTrait;
    use hitbox_http::BufferedBody;
    use http::Response;
    use http_body_util::Full;

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

    #[test]
    fn static_round_trips_via_yaml() {
        let yaml = r#"
            Static:
              etag: "v1"
        "#;
        let parsed: ResponseTagExtractor = serde_saphyr::from_str(yaml).unwrap();
        assert!(matches!(parsed, ResponseTagExtractor::Static(_)));
    }

    #[test]
    fn header_round_trips_via_yaml() {
        let yaml = r#"Header: "etag""#;
        let parsed: ResponseTagExtractor = serde_saphyr::from_str(yaml).unwrap();
        assert!(matches!(parsed, ResponseTagExtractor::Header(_)));
    }

    #[tokio::test]
    async fn header_variant_emits_tag_from_response() {
        let configs = vec![ResponseTagExtractor::Header(HeaderOperation::Simple(
            "etag".into(),
        ))];
        let ext = build_response_boxed::<Full<Bytes>>(configs).unwrap();
        let (_subject, tags) = ext.extract_tags(response_with(&[("etag", "abc123")])).await;
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].as_str(), "etag=abc123");
    }

    #[tokio::test]
    async fn static_and_header_chain_concatenate() {
        let mut map = indexmap::IndexMap::new();
        map.insert("tier".to_string(), "premium".to_string());
        let configs = vec![
            ResponseTagExtractor::Static(Static::Map(map)),
            ResponseTagExtractor::Header(HeaderOperation::Simple("etag".into())),
        ];
        let ext = build_response_boxed::<Full<Bytes>>(configs).unwrap();
        let (_subject, tags) = ext.extract_tags(response_with(&[("etag", "abc123")])).await;
        let strs: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
        assert_eq!(strs, vec!["tier=premium", "etag=abc123"]);
    }
}
