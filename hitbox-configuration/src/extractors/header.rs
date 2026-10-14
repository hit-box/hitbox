//! Header extractor configuration.
//!
//! Supports various ways to select headers and extract values:
//!
//! ```yaml
//! extractors:
//!   # Simple
//!   - Header: "Authorization"
//!
//!   # With value regex extraction
//!   - Header:
//!       name: "Authorization"
//!       value: "Bearer (.+)"
//!
//!   # With transforms
//!   - Header:
//!       name: "Authorization"
//!       transforms: [hash]
//!
//!   # With transform chain
//!   - Header:
//!       name: "Authorization"
//!       transforms: [lowercase, hash]
//!
//!   # Prefix match + regex + transforms
//!   - Header:
//!       name:
//!         starts: "X-API-"
//!       value:
//!         regex: "key=(.+)"
//!       transforms: [hash]
//! ```

use hitbox_http::extractors::header::{
    HeaderConfig as HttpHeaderConfig, HeaderExtractor, NameSelector as HttpNameSelector,
    ValueExtractor as HttpValueExtractor,
};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::transform::{Transform, into_http_transforms};
use crate::RequestExtractor;
use crate::error::ConfigError;

/// Header name selector configuration.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum NameSelector {
    /// Exact name match (implicit): `name: "Authorization"`
    Exact(String),
    /// Explicit operation
    Operation(NameOperation),
}

/// Explicit name selection operations.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum NameOperation {
    /// Exact match: `name: { eq: "Authorization" }`
    Eq(String),
    /// Prefix match: `name: { starts: "X-Custom-" }`
    Starts(String),
}

/// Value extractor configuration.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum ValueExtractor {
    /// Implicit regex: `value: "Bearer (.+)"`
    Regex(String),
    /// Explicit operation
    Operation(ValueOperation),
}

/// Explicit value extraction operations.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ValueOperation {
    /// Regex extraction: `value: { regex: "Bearer (.+)" }`
    Regex(String),
}

/// Extended header configuration.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct HeaderConfig {
    /// Header name selector
    pub name: NameSelector,
    /// Value extractor (optional, defaults to full value)
    #[serde(default)]
    pub value: Option<ValueExtractor>,
    /// Value transformations (optional)
    #[serde(default)]
    pub transforms: Vec<Transform>,
}

/// Header extractor operation.
///
/// Supports both simple string (backwards compatible) and extended configuration.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum HeaderOperation {
    /// Simple header name: `Header: "Authorization"`
    Simple(String),
    /// Extended configuration
    Extended(HeaderConfig),
}

impl HeaderOperation {
    /// Resolve into a runtime [`HttpHeaderConfig`] (name selector, value
    /// extractor, transforms).
    ///
    /// Shared by the request key extractor ([`into_extractors`]) and the
    /// response header tag extractor, so the config→runtime mapping lives in
    /// one place.
    ///
    /// [`into_extractors`]: HeaderOperation::into_extractors
    pub fn into_header_config(self) -> Result<HttpHeaderConfig, ConfigError> {
        let (name_selector, value_extractor, transforms) = match self {
            HeaderOperation::Simple(name) => (
                HttpNameSelector::Exact(name),
                HttpValueExtractor::Full,
                Vec::new(),
            ),
            HeaderOperation::Extended(config) => {
                let name_selector = match config.name {
                    NameSelector::Exact(name) => HttpNameSelector::Exact(name),
                    NameSelector::Operation(op) => match op {
                        NameOperation::Eq(name) => HttpNameSelector::Exact(name),
                        NameOperation::Starts(prefix) => HttpNameSelector::Starts(prefix),
                    },
                };

                let value_extractor = match config.value {
                    None => HttpValueExtractor::Full,
                    Some(ValueExtractor::Regex(pattern)) => {
                        let regex = compile_regex(&pattern)?;
                        HttpValueExtractor::Regex(regex)
                    }
                    Some(ValueExtractor::Operation(ValueOperation::Regex(pattern))) => {
                        let regex = compile_regex(&pattern)?;
                        HttpValueExtractor::Regex(regex)
                    }
                };

                let transforms = into_http_transforms(config.transforms);

                (name_selector, value_extractor, transforms)
            }
        };

        Ok(HttpHeaderConfig::new(
            name_selector,
            value_extractor,
            transforms,
        ))
    }

    pub fn into_extractors<ReqBody>(
        self,
        inner: RequestExtractor<ReqBody>,
    ) -> Result<RequestExtractor<ReqBody>, ConfigError>
    where
        ReqBody: hyper::body::Body + Send + 'static,
        ReqBody::Error: Send,
        ReqBody::Data: Send,
    {
        let config = self.into_header_config()?;
        Ok(Box::new(inner.header(config)))
    }
}

fn compile_regex(pattern: &str) -> Result<Regex, ConfigError> {
    Regex::new(pattern).map_err(|e| ConfigError::InvalidRegex {
        pattern: pattern.to_string(),
        error: e,
    })
}
