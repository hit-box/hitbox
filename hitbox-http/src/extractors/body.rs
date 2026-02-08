//! Body content extraction for cache keys.
//!
//! Provides [`Body`] extractor with support for hashing, jq (JSON) queries,
//! and regular expression matching.
//!
//! # Extraction Modes
//!
//! - **Hash**: SHA256 hash of the entire body (truncated to 16 hex chars)
//! - **Jq**: Extract values from JSON bodies using jq expressions
//! - **Regex**: Extract values using regular expression capture groups
//!
//! # Performance
//!
//! All modes buffer the body into memory. For large bodies, consider
//! using hash mode to minimize cache key size.

use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};
use hyper::body::Body as HttpBody;
use jaq_core::box_iter::box_once;
use jaq_core::load::{Arena, File, Loader};
use jaq_core::{Bind, Ctx, Exn, Filter, Native, RcIter};
use jaq_json::Val;
use regex::Regex;
use serde_json::Value;

pub use super::transform::Transform;
use super::transform::{apply_hash, apply_transform_chain};
use crate::CacheableHttpRequest;

/// Body extraction mode for generating cache key parts.
///
/// # Variants
///
/// - [`Hash`](Self::Hash): SHA256 hash of entire body
/// - [`Jq`](Self::Jq): Extract from JSON using jq expressions
/// - [`Regex`](Self::Regex): Extract using regular expression captures
#[derive(Debug, Clone)]
pub enum BodyExtraction {
    /// Hash the entire body using SHA256 (truncated to 16 hex chars).
    Hash,
    /// Extract values from JSON body using a jq expression.
    Jq(JqExtraction),
    /// Extract values using regular expression captures.
    Regex(RegexExtraction),
}

/// A compiled jq expression for extracting values from JSON bodies.
///
/// Includes a custom `hash` function for hashing extracted values.
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::body::JqExtraction;
///
/// // Extract user ID from JSON body
/// let extraction = JqExtraction::compile(".user.id").unwrap();
///
/// // Extract and hash a sensitive field
/// let extraction = JqExtraction::compile(".password | hash").unwrap();
/// ```
#[derive(Clone)]
pub struct JqExtraction {
    filter: Filter<Native<Val>>,
}

impl Debug for JqExtraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JqExtraction").finish_non_exhaustive()
    }
}

/// Result type for jq functions.
type JqResult = Result<Val, jaq_core::Error<Val>>;

/// Custom jq functions for hitbox.
fn custom_jq_funs() -> impl Iterator<Item = (&'static str, Box<[Bind]>, Native<Val>)> {
    let v0: Box<[Bind]> = Box::new([]);

    [
        // hash: SHA256 hash of the string value (truncated to 16 hex chars)
        (
            "hash",
            v0,
            Native::new(|_, cv| {
                let val = cv.1;
                let result: JqResult = match &val {
                    Val::Str(s) => {
                        let hash = apply_hash(s);
                        Ok(Val::Str(Rc::new(hash)))
                    }
                    Val::Int(n) => {
                        let hash = apply_hash(&n.to_string());
                        Ok(Val::Str(Rc::new(hash)))
                    }
                    Val::Float(f) => {
                        let hash = apply_hash(&f.to_string());
                        Ok(Val::Str(Rc::new(hash)))
                    }
                    Val::Bool(b) => {
                        let hash = apply_hash(&b.to_string());
                        Ok(Val::Str(Rc::new(hash)))
                    }
                    Val::Null => {
                        let hash = apply_hash("null");
                        Ok(Val::Str(Rc::new(hash)))
                    }
                    Val::Num(n) => {
                        let hash = apply_hash(n);
                        Ok(Val::Str(Rc::new(hash)))
                    }
                    Val::Arr(_) | Val::Obj(_) => {
                        // For arrays and objects, serialize to JSON string first
                        let json: Value = val.clone().into();
                        let hash = apply_hash(&json.to_string());
                        Ok(Val::Str(Rc::new(hash)))
                    }
                };
                box_once(result.map_err(Exn::from))
            }),
        ),
    ]
    .into_iter()
}

impl JqExtraction {
    /// Compiles a jq expression for extracting values from JSON bodies.
    ///
    /// The compiled filter can be reused across multiple requests.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` if the expression is invalid:
    /// - Parse errors (syntax errors in the jq expression)
    /// - Compile errors (undefined functions, type mismatches)
    ///
    /// The error message includes details about the parsing or compilation failure.
    pub fn compile(expression: &str) -> Result<Self, String> {
        let program = File {
            code: expression,
            path: (),
        };
        let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()));
        let arena = Arena::default();
        let modules = loader
            .load(&arena, program)
            .map_err(|e| format!("jq parse error: {:?}", e))?;
        let filter = jaq_core::Compiler::default()
            .with_funs(
                jaq_std::funs()
                    .chain(jaq_json::funs())
                    .chain(custom_jq_funs()),
            )
            .compile(modules)
            .map_err(|e| format!("jq compile error: {:?}", e))?;
        Ok(Self { filter })
    }

    fn apply(&self, input: Value) -> Vec<Value> {
        let inputs = RcIter::new(core::iter::empty());
        let out = self.filter.run((Ctx::new([], &inputs), Val::from(input)));
        out.filter_map(|r| r.ok()).map(|v| v.into()).collect()
    }
}

/// Configuration for regex-based body extraction.
///
/// Extracts values using regular expression captures. Supports both named
/// and unnamed capture groups, with optional transformations.
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::body::{RegexExtraction, Transforms};
/// use regex::Regex;
///
/// // Extract order ID from body
/// let extraction = RegexExtraction {
///     regex: Regex::new(r#""order_id":\s*"(\w+)""#).unwrap(),
///     key: Some("order_id".to_string()),
///     global: false,
///     transforms: Transforms::None,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct RegexExtraction {
    /// The regular expression pattern.
    pub regex: Regex,
    /// Key name for unnamed captures. Defaults to `"body"` if `None`.
    pub key: Option<String>,
    /// If `true`, extract all matches; if `false`, extract first match only.
    pub global: bool,
    /// Transformations to apply to captured values.
    pub transforms: Transforms,
}

/// Transformations to apply to extracted values.
///
/// Apply hash, lowercase, or other transforms to captured values
/// before using them in cache keys.
#[derive(Debug, Clone, Default)]
pub enum Transforms {
    /// No transformations applied.
    #[default]
    None,
    /// Apply transforms to all captured values.
    FullBody(Vec<Transform>),
    /// Apply different transforms per capture group name.
    PerKey(HashMap<String, Vec<Transform>>),
}

/// Extracts cache key parts from request bodies.
///
/// Supports hash, jq (JSON), and regex extraction modes.
/// Chain with other extractors using the builder pattern.
///
/// # Caveats
///
/// The entire body is buffered into memory during extraction.
/// The body is returned as [`BufferedBody::Complete`](crate::BufferedBody::Complete)
/// after extraction.
#[derive(Debug)]
pub struct Body<E> {
    inner: E,
    extraction: BodyExtraction,
}

impl<S> Body<super::NeutralExtractor<S>> {
    /// Creates a body extractor with the given extraction mode.
    pub fn new(extraction: BodyExtraction) -> Self {
        Self {
            inner: super::NeutralExtractor::new(),
            extraction,
        }
    }
}

/// Extension trait for adding body extraction to an extractor chain.
///
/// # For Callers
///
/// Chain this to extract cache key parts from request bodies. Choose an
/// extraction mode based on your needs:
/// - [`BodyExtraction::Hash`] for opaque body identification
/// - [`BodyExtraction::Jq`] for JSON content extraction
/// - [`BodyExtraction::Regex`] for pattern-based extraction
///
/// **Important**: Body extraction buffers the entire body into memory.
/// The body is returned as [`BufferedBody::Complete`](crate::BufferedBody::Complete) after extraction.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Extractor`]
/// types. You don't need to implement it manually.
pub trait BodyExtractor: Sized {
    /// Adds body extraction with the specified mode.
    fn body(self, extraction: BodyExtraction) -> Body<Self>;
}

impl<E> BodyExtractor for E
where
    E: hitbox::Extractor,
{
    fn body(self, extraction: BodyExtraction) -> Body<Self> {
        Body {
            inner: self,
            extraction,
        }
    }
}

/// Extract key parts from jq result.
fn extract_jq_parts(values: Vec<Value>) -> Vec<KeyPart> {
    let mut parts = Vec::new();

    for value in values {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    let value_str = value_to_string(&val);
                    parts.push(KeyPart::new(key, value_str));
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    match item {
                        Value::Object(map) => {
                            for (key, val) in map {
                                let value_str = value_to_string(&val);
                                parts.push(KeyPart::new(key, value_str));
                            }
                        }
                        other => {
                            let value_str = value_to_string(&other);
                            parts.push(KeyPart::new("body", value_str));
                        }
                    }
                }
            }
            other => {
                let value_str = value_to_string(&other);
                parts.push(KeyPart::new("body", value_str));
            }
        }
    }

    parts
}

/// Convert JSON value to string for cache key.
fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        other => Some(other.to_string()),
    }
}

/// Extract key parts from regex matches.
fn extract_regex_parts(
    body: &str,
    regex: &Regex,
    key: &Option<String>,
    global: bool,
    transforms: &Transforms,
) -> Vec<KeyPart> {
    let mut parts = Vec::new();
    let capture_names: Vec<_> = regex.capture_names().flatten().collect();
    let has_named_groups = !capture_names.is_empty();

    let apply_transforms = |key_name: &str, value: String| -> String {
        match transforms {
            Transforms::None => value,
            Transforms::FullBody(chain) => apply_transform_chain(value, chain),
            Transforms::PerKey(map) => {
                if let Some(chain) = map.get(key_name) {
                    apply_transform_chain(value, chain)
                } else {
                    value
                }
            }
        }
    };

    if global {
        for caps in regex.captures_iter(body) {
            if has_named_groups {
                for name in &capture_names {
                    if let Some(m) = caps.name(name) {
                        let value = apply_transforms(name, m.as_str().to_string());
                        parts.push(KeyPart::new(*name, Some(value)));
                    }
                }
            } else if let Some(m) = caps.get(1).or_else(|| caps.get(0)) {
                let key_name = key.as_deref().unwrap_or("body");
                let value = apply_transforms(key_name, m.as_str().to_string());
                parts.push(KeyPart::new(key_name, Some(value)));
            }
        }
    } else if let Some(caps) = regex.captures(body) {
        if has_named_groups {
            for name in &capture_names {
                if let Some(m) = caps.name(name) {
                    let value = apply_transforms(name, m.as_str().to_string());
                    parts.push(KeyPart::new(*name, Some(value)));
                }
            }
        } else if let Some(m) = caps.get(1).or_else(|| caps.get(0)) {
            let key_name = key.as_deref().unwrap_or("body");
            let value = apply_transforms(key_name, m.as_str().to_string());
            parts.push(KeyPart::new(key_name, Some(value)));
        }
    }

    parts
}

#[async_trait]
impl<ReqBody, E> Extractor for Body<E>
where
    ReqBody: HttpBody + Send + 'static,
    ReqBody::Error: Send,
    ReqBody::Data: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let (parts, body) = subject.into_parts();

        // Collect body
        let payload = match body.collect().await {
            Ok(bytes) => bytes,
            Err(error_body) => {
                let request = CacheableHttpRequest::from_request(http::Request::from_parts(
                    parts, error_body,
                ));
                let mut key_parts = self.inner.get(request).await;
                key_parts.push(KeyPart::new("body", None::<String>));
                return key_parts;
            }
        };

        let body_bytes = payload.to_vec();
        let body_str = String::from_utf8_lossy(&body_bytes);

        let extracted_parts = match &self.extraction {
            BodyExtraction::Hash => {
                let hash = apply_hash(&body_str);
                vec![KeyPart::new("body", Some(hash))]
            }
            BodyExtraction::Jq(jq) => {
                let json_value = serde_json::from_str(&body_str).unwrap_or(Value::Null);
                let results = jq.apply(json_value);
                extract_jq_parts(results)
            }
            BodyExtraction::Regex(regex_ext) => extract_regex_parts(
                &body_str,
                &regex_ext.regex,
                &regex_ext.key,
                regex_ext.global,
                &regex_ext.transforms,
            ),
        };

        let body = crate::BufferedBody::Complete(Some(payload));
        let request = CacheableHttpRequest::from_request(http::Request::from_parts(parts, body));

        let mut key_parts = self.inner.get(request).await;
        for part in extracted_parts {
            key_parts.push(part);
        }
        key_parts
    }
}
