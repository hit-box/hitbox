//! Body content extraction for cache keys.
//!
//! Provides [`Body`] extractor with support for hashing, jq (JSON) queries,
//! and regular expression matching.
//!
//! # Extraction Modes
//!
//! - **Hash**: Full SHA256 hash of the entire body (64 hex characters)
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
use tracing::warn;

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
    /// Hash the entire body using SHA256 (full 64 hex characters).
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
///
/// Use [`Transforms::builder()`] for ergonomic construction:
///
/// ```
/// use hitbox_http::extractors::body::Transforms;
/// use hitbox_http::extractors::transform::Transform;
///
/// let t: Transforms = Transforms::builder()
///     .full(Transform::Hash)
///     .full(Transform::Truncate(16))
///     .into();
/// ```
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

impl Transforms {
    /// Create a [`TransformBuilder`] for ergonomic construction.
    pub fn builder() -> TransformBuilder<BuilderEmpty> {
        TransformBuilder {
            state: BuilderEmpty,
        }
    }
}

/// Builder for [`Transforms`], created via [`Transforms::builder()`].
///
/// Uses typestate to enforce at compile time that `.full()` and `.key()`
/// cannot be mixed.
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::body::Transforms;
/// use hitbox_http::extractors::transform::Transform;
///
/// // Full-body transform chain
/// let transforms = Transforms::builder()
///     .full(Transform::Hash)
///     .full(Transform::Truncate(16));
///
/// // Per-key transforms
/// let transforms = Transforms::builder()
///     .key("token", Transform::Hash)
///     .key("name", Transform::Lowercase);
/// ```
#[derive(Debug, Clone)]
pub struct TransformBuilder<S> {
    state: S,
}

/// Initial state: no transforms added yet.
#[derive(Debug, Clone)]
pub struct BuilderEmpty;

/// State after adding full-body transforms.
#[derive(Debug, Clone)]
pub struct BuilderFull(Vec<Transform>);

/// State after adding per-key transforms.
#[derive(Debug, Clone)]
pub struct BuilderPerKey(HashMap<String, Vec<Transform>>);

impl TransformBuilder<BuilderEmpty> {
    /// Add a transform applied to all captured values.
    ///
    /// Transitions to full-body mode. Only `.full()` can be called after this.
    pub fn full(self, t: Transform) -> TransformBuilder<BuilderFull> {
        TransformBuilder {
            state: BuilderFull(vec![t]),
        }
    }

    /// Add a transform for a specific capture group by name.
    ///
    /// Transitions to per-key mode. Only `.key()` can be called after this.
    pub fn key(self, name: impl Into<String>, t: Transform) -> TransformBuilder<BuilderPerKey> {
        let mut map = HashMap::new();
        map.insert(name.into(), vec![t]);
        TransformBuilder {
            state: BuilderPerKey(map),
        }
    }
}

impl TransformBuilder<BuilderFull> {
    /// Add another transform to the full-body chain.
    pub fn full(mut self, t: Transform) -> Self {
        self.state.0.push(t);
        self
    }
}

impl TransformBuilder<BuilderPerKey> {
    /// Add a transform for a specific capture group by name.
    ///
    /// Multiple calls for the same key chain transforms in order.
    pub fn key(mut self, name: impl Into<String>, t: Transform) -> Self {
        self.state.0.entry(name.into()).or_default().push(t);
        self
    }
}

impl From<TransformBuilder<BuilderEmpty>> for Transforms {
    fn from(_: TransformBuilder<BuilderEmpty>) -> Self {
        Transforms::None
    }
}

impl From<TransformBuilder<BuilderFull>> for Transforms {
    fn from(builder: TransformBuilder<BuilderFull>) -> Self {
        Transforms::FullBody(builder.state.0)
    }
}

impl From<TransformBuilder<BuilderPerKey>> for Transforms {
    fn from(builder: TransformBuilder<BuilderPerKey>) -> Self {
        Transforms::PerKey(builder.state.0)
    }
}

/// Trait for converting a body extraction config into a [`BodyExtraction`].
pub trait IntoBodyExtraction {
    /// Convert into a [`BodyExtraction`].
    fn into_extraction(self) -> BodyExtraction;
}

impl IntoBodyExtraction for BodyExtraction {
    fn into_extraction(self) -> BodyExtraction {
        self
    }
}

/// Initial state for [`BodyConfig`] before a mode is chosen.
#[derive(Debug, Clone)]
pub struct NoMode;

/// Hash mode for [`BodyConfig`].
#[derive(Debug, Clone)]
pub struct HashMode;

/// JQ mode for [`BodyConfig`].
#[derive(Debug, Clone)]
pub struct JqMode {
    extraction: JqExtraction,
}

/// Regex mode for [`BodyConfig`].
#[derive(Debug, Clone)]
pub struct RegexMode {
    regex: Regex,
    global: bool,
}

/// Configuration for the body extractor, parameterized by extraction mode.
///
/// Start with [`BodyConfig::new()`] to set shared options (key, transforms),
/// then choose a mode. Or use the mode constructors directly as shortcuts.
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::{self, MethodConfig, MethodExtractor, body::{BodyConfig, BodyExtractor}};
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// // Hash mode
/// let extractor = extractors::extractor::<Empty<Bytes>>()
///     .method(MethodConfig::new())
///     .body(BodyConfig::new().hash());
///
/// // Shared config then mode
/// let extractor = extractors::extractor::<Empty<Bytes>>()
///     .method(MethodConfig::new())
///     .body(BodyConfig::new().key("token").regex(r"(\w+)").unwrap().global());
///
/// // Mode-first shortcut with key
/// let extractor = extractors::extractor::<Empty<Bytes>>()
///     .method(MethodConfig::new())
///     .body(BodyConfig::new().regex(r"token=(\w+)").unwrap().key("api-token").global());
/// ```
#[derive(Debug, Clone)]
pub struct BodyConfig<M> {
    key: Option<String>,
    transforms: Transforms,
    mode: M,
}

// Shared builder methods available on all states.
impl<M> BodyConfig<M> {
    /// Set the key name for generated key parts. Defaults to `"body"`.
    ///
    /// Behavior varies by mode:
    /// - **Hash**: used as the key part name (replaces default `"body"`).
    /// - **Regex**: used as the key part name for unnamed capture groups.
    ///   Named captures use their own names regardless of this setting.
    /// - **Jq**: used as the key part name for scalar and array-of-scalar
    ///   outputs. Object outputs use their field names, and array-of-object
    ///   outputs use each object's field names — this setting is ignored
    ///   in those cases.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Set transforms for extracted values.
    ///
    /// Accepts [`Transforms`] directly or a [`TransformBuilder`].
    pub fn transforms(mut self, transforms: impl Into<Transforms>) -> Self {
        self.transforms = transforms.into();
        self
    }
}

impl BodyConfig<NoMode> {
    /// Create a new body config without a mode. Set shared options,
    /// then choose a mode with `.hash()`, `.jq()`, or `.regex()`.
    pub fn new() -> Self {
        BodyConfig {
            key: None,
            transforms: Transforms::None,
            mode: NoMode,
        }
    }

    /// Switch to hash mode.
    pub fn hash(self) -> BodyConfig<HashMode> {
        BodyConfig {
            key: self.key,
            transforms: self.transforms,
            mode: HashMode,
        }
    }

    /// Switch to jq mode.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the jq expression is invalid.
    pub fn jq(self, expression: &str) -> Result<BodyConfig<JqMode>, String> {
        Ok(BodyConfig {
            key: self.key,
            transforms: self.transforms,
            mode: JqMode {
                extraction: JqExtraction::compile(expression)?,
            },
        })
    }

    /// Switch to regex mode.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the regex pattern is invalid.
    pub fn regex(self, pattern: &str) -> Result<BodyConfig<RegexMode>, regex::Error> {
        Ok(BodyConfig {
            key: self.key,
            transforms: self.transforms,
            mode: RegexMode {
                regex: Regex::new(pattern)?,
                global: false,
            },
        })
    }
}

impl Default for BodyConfig<NoMode> {
    fn default() -> Self {
        Self::new()
    }
}

impl BodyConfig<RegexMode> {
    /// Enable global matching (extract all matches instead of first).
    pub fn global(mut self) -> Self {
        self.mode.global = true;
        self
    }
}

impl IntoBodyExtraction for BodyConfig<HashMode> {
    fn into_extraction(self) -> BodyExtraction {
        BodyExtraction::Hash
    }
}

impl IntoBodyExtraction for BodyConfig<JqMode> {
    fn into_extraction(self) -> BodyExtraction {
        BodyExtraction::Jq(self.mode.extraction)
    }
}

impl IntoBodyExtraction for BodyConfig<RegexMode> {
    fn into_extraction(self) -> BodyExtraction {
        BodyExtraction::Regex(RegexExtraction {
            regex: self.mode.regex,
            key: self.key,
            global: self.mode.global,
            transforms: self.transforms,
        })
    }
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

/// Extension trait for adding body extraction to an extractor chain.
///
/// # For Callers
///
/// Chain this to extract cache key parts from request bodies. Choose an
/// extraction mode via [`BodyConfig`]:
/// - [`BodyConfig::hash()`] for opaque body identification
/// - [`BodyConfig::jq()`] for JSON content extraction
/// - [`BodyConfig::regex()`] for pattern-based extraction
///
/// **Important**: Body extraction buffers the entire body into memory.
/// The body is returned as [`BufferedBody::Complete`](crate::BufferedBody::Complete) after extraction.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Extractor`]
/// types. You don't need to implement it manually.
pub trait BodyExtractor: Sized {
    /// Adds body extraction with the specified configuration.
    fn body(self, config: impl IntoBodyExtraction) -> Body<Self>;
}

impl<E> BodyExtractor for E
where
    E: hitbox::Extractor,
{
    fn body(self, config: impl IntoBodyExtraction) -> Body<Self> {
        Body {
            inner: self,
            extraction: config.into_extraction(),
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
    type Context = E::Context;

    async fn get(
        &self,
        subject: Self::Subject,
        ctx: &mut Self::Context,
    ) -> KeyParts<Self::Subject> {
        let (parts, body) = subject.into_parts();

        // Collect body
        let collected = match body.collect().await {
            Ok(c) => c,
            Err(error_body) => {
                let request = CacheableHttpRequest::from_request(http::Request::from_parts(
                    parts, error_body,
                ));
                let mut key_parts = self.inner.get(request, ctx).await;
                key_parts.push(KeyPart::new("body", None::<String>));
                return key_parts;
            }
        };
        let (payload, payload_trailers) = (collected.data, collected.trailers);

        let body_bytes = payload.to_vec();
        let body_str = String::from_utf8_lossy(&body_bytes);

        let extracted_parts = match &self.extraction {
            BodyExtraction::Hash => {
                let hash = apply_hash(&body_str);
                vec![KeyPart::new("body", Some(hash))]
            }
            BodyExtraction::Jq(jq) => match serde_json::from_str(&body_str) {
                Ok(json_value) => {
                    let results = jq.apply(json_value);
                    extract_jq_parts(results)
                }
                Err(err) => {
                    warn!(%err, "Jq body extraction failed: invalid JSON, falling back to body hash");
                    let hash = apply_hash(&body_str);
                    vec![KeyPart::new("body", Some(hash))]
                }
            },
            BodyExtraction::Regex(regex_ext) => extract_regex_parts(
                &body_str,
                &regex_ext.regex,
                &regex_ext.key,
                regex_ext.global,
                &regex_ext.transforms,
            ),
        };

        let body = crate::BufferedBody::Complete {
            data: Some(payload),
            trailers: payload_trailers,
        };
        let request = CacheableHttpRequest::from_request(http::Request::from_parts(parts, body));

        let mut key_parts = self.inner.get(request, ctx).await;
        for part in extracted_parts {
            key_parts.push(part);
        }
        key_parts
    }
}
