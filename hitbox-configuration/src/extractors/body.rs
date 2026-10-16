use std::collections::HashMap;

use hitbox_http::extractors;
use hitbox_http::extractors::body::BodyExtractor;
use hyper::body::Body as HttpBody;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::transform::Transform;
use crate::{ConfigError, RequestExtractor};

/// Body extractor configuration.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum BodyOperation {
    /// Full body transforms: `transforms: [hash]`
    Transforms(TransformsOperation),
    /// Jq extraction (JSON)
    Jq(JqOperation),
    /// Regex extraction (plain text)
    Regex(RegexOperation),
}

/// Full body transforms operation (only hash supported)
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct TransformsOperation {
    pub transforms: Vec<BodyTransform>,
}

/// Body-level transforms (only hash for predictable key length)
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum BodyTransform {
    Hash,
}

/// Jq extraction configuration
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct JqOperation {
    pub jq: String,
}

/// Regex extraction configuration
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct RegexOperation {
    pub regex: String,
    /// Key name for unnamed capture groups
    #[serde(default)]
    pub key: Option<String>,
    /// Match all occurrences (global matching)
    #[serde(default)]
    pub global: bool,
    /// Transformations: per-key or full body
    #[serde(default)]
    pub transforms: Transforms,
}

/// Transformations configuration
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
#[serde(untagged)]
pub enum Transforms {
    /// Full body transform chain: `transforms: [hash, lowercase]`
    FullBody(Vec<Transform>),
    /// Per-key transforms: `transforms: {token: hash}` or `transforms: {token: [hash]}`
    PerKey(HashMap<String, TransformChain>),
    #[default]
    None,
}

/// Single transform or chain of transforms
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum TransformChain {
    /// Single transform: `token: hash`
    Single(Transform),
    /// Chain of transforms: `token: [lowercase, hash]`
    Chain(Vec<Transform>),
}

impl TransformChain {
    pub fn into_vec(self) -> Vec<Transform> {
        match self {
            TransformChain::Single(t) => vec![t],
            TransformChain::Chain(v) => v,
        }
    }
}

impl BodyOperation {
    pub fn into_extractors<ReqBody>(
        self,
        inner: RequestExtractor<ReqBody>,
    ) -> Result<RequestExtractor<ReqBody>, ConfigError>
    where
        ReqBody: HttpBody + Send + 'static,
        ReqBody::Error: std::fmt::Debug + Send,
        ReqBody::Data: Send,
    {
        match self {
            BodyOperation::Transforms(_) => {
                // Currently only hash is supported
                Ok(Box::new(inner.body(extractors::body::BodyExtraction::Hash)))
            }
            BodyOperation::Jq(jq_op) => Ok(Box::new(
                inner.body(extractors::body::BodyExtraction::Jq(
                    extractors::body::JqExpression::compile(&jq_op.jq)
                        .map_err(ConfigError::InvalidPredicate)?,
                )),
            )),
            BodyOperation::Regex(regex_op) => {
                let regex = Regex::new(&regex_op.regex).map_err(|e| ConfigError::InvalidRegex {
                    pattern: regex_op.regex.clone(),
                    error: e,
                })?;
                let transforms = match regex_op.transforms {
                    Transforms::None => extractors::body::Transforms::None,
                    Transforms::FullBody(chain) => extractors::body::Transforms::FullBody(
                        chain.into_iter().map(Transform::into_http).collect(),
                    ),
                    Transforms::PerKey(map) => extractors::body::Transforms::PerKey(
                        map.into_iter()
                            .map(|(k, v)| {
                                let chain =
                                    v.into_vec().into_iter().map(Transform::into_http).collect();
                                (k, chain)
                            })
                            .collect(),
                    ),
                };
                Ok(Box::new(inner.body(
                    extractors::body::BodyExtraction::Regex(extractors::body::RegexExtraction {
                        regex,
                        key: regex_op.key,
                        global: regex_op.global,
                        transforms,
                    }),
                )))
            }
        }
    }
}
