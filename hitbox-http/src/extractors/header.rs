//! Header extraction for cache keys.
//!
//! Provides [`Header`] extractor with support for name selection, value extraction,
//! and transformation.
//!
//! # Examples
//!
//! Extract a single header:
//!
//! ```
//! use hitbox_http::extractors::{self, MethodConfig, MethodExtractor};
//! use hitbox_http::extractors::header::HeaderExtractor;
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! # use hitbox_http::extractors::{NeutralExtractor, Method, header::Header};
//! let extractor = extractors::extractor::<Empty<Bytes>>()
//!     .method(MethodConfig::new())
//!     .header("x-api-key");
//! # let _: &Header<Method<NeutralExtractor<Empty<Bytes>>>> = &extractor;
//! ```

use async_trait::async_trait;
use hitbox::EvalContext;
use hitbox::{Extractor, KeyPart, KeyParts};
use http::HeaderValue;
use regex::Regex;

pub use super::transform::Transform;
use super::transform::apply_transform_chain;
use crate::CacheableHttpRequest;

/// Selects which headers to extract.
#[derive(Debug, Clone)]
pub enum NameSelector {
    /// Match a single header by exact name.
    Exact(String),
    /// Match all headers starting with a prefix.
    ///
    /// Results are sorted by header name for deterministic cache keys.
    Starts(String),
}

impl NameSelector {
    /// Select by exact name.
    pub fn exact(name: impl Into<String>) -> Self {
        NameSelector::Exact(name.into())
    }

    /// Select by name prefix.
    pub fn starts(prefix: impl Into<String>) -> Self {
        NameSelector::Starts(prefix.into())
    }
}

/// Extracts values from header content.
#[derive(Debug, Clone)]
pub enum ValueExtractor {
    /// Use the full header value.
    Full,
    /// Extract using regex (returns first capture group, or full match if no groups).
    Regex(Regex),
}

/// Configuration for the header extractor.
///
/// Builder with name selection, value extraction, and transform options.
///
/// # Examples
///
/// ```
/// use hitbox_http::extractors::{self, MethodConfig, MethodExtractor, header::{HeaderConfig, HeaderExtractor, NameSelector}};
/// use hitbox_http::extractors::transform::Transform;
///
/// # use bytes::Bytes;
/// # use http_body_util::Empty;
/// let extractor = extractors::extractor::<Empty<Bytes>>()
///     .method(MethodConfig::new())
///     .header(HeaderConfig::name(NameSelector::exact("x-api-key")))
///     .header(HeaderConfig::name(NameSelector::exact("accept")).transform(Transform::Lowercase));
/// ```
#[derive(Debug, Clone)]
pub struct HeaderConfig {
    pub(crate) name_selector: NameSelector,
    pub(crate) value_extractor: ValueExtractor,
    pub(crate) transforms: Vec<Transform>,
}

impl HeaderConfig {
    /// Create a header extractor configuration with the given name selector.
    pub fn name(selector: NameSelector) -> Self {
        HeaderConfig {
            name_selector: selector,
            value_extractor: ValueExtractor::Full,
            transforms: Vec::new(),
        }
    }

    /// Set value extraction strategy.
    pub fn value(mut self, extractor: ValueExtractor) -> Self {
        self.value_extractor = extractor;
        self
    }

    /// Add a transform to the chain.
    pub fn transform(mut self, t: Transform) -> Self {
        self.transforms.push(t);
        self
    }
}

impl From<&str> for HeaderConfig {
    fn from(name: &str) -> Self {
        HeaderConfig::name(NameSelector::exact(name))
    }
}

impl From<String> for HeaderConfig {
    fn from(name: String) -> Self {
        HeaderConfig::name(NameSelector::exact(name))
    }
}

/// Extracts header values as cache key parts.
///
/// Supports flexible header selection, value extraction, and transformation.
///
/// # Key Parts Generated
///
/// For each matched header, generates a `KeyPart` with:
/// - Key: the header name
/// - Value: the extracted (and optionally transformed) value
#[derive(Debug)]
pub struct Header<E> {
    inner: E,
    name_selector: NameSelector,
    value_extractor: ValueExtractor,
    transforms: Vec<Transform>,
}

impl<E> Header<E> {
    /// Creates a header extractor with full configuration options.
    ///
    /// This constructor provides complete control over header extraction:
    /// - Select headers by exact name or prefix pattern
    /// - Extract full values or use regex capture groups
    /// - Apply transformations (hash, lowercase, uppercase)
    ///
    /// For simple exact-name extraction without transforms, use
    /// [`HeaderExtractor::header`] instead.
    pub fn new_with(
        inner: E,
        name_selector: NameSelector,
        value_extractor: ValueExtractor,
        transforms: Vec<Transform>,
    ) -> Self {
        Self {
            inner,
            name_selector,
            value_extractor,
            transforms,
        }
    }
}

/// Extension trait for adding header extraction to an extractor chain.
///
/// # For Callers
///
/// Chain this to extract HTTP header values as cache key parts. The header
/// name becomes the key part name, and the header value becomes the key part value.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Extractor`]
/// types. You don't need to implement it manually.
pub trait HeaderExtractor: Sized {
    /// Adds header extraction with the given configuration.
    ///
    /// Accepts a [`HeaderConfig`] or a string (exact header name) directly.
    fn header(self, config: impl Into<HeaderConfig>) -> Header<Self>;
}

impl<E> HeaderExtractor for E
where
    E: Extractor,
{
    fn header(self, config: impl Into<HeaderConfig>) -> Header<Self> {
        let config = config.into();
        Header {
            inner: self,
            name_selector: config.name_selector,
            value_extractor: config.value_extractor,
            transforms: config.transforms,
        }
    }
}

/// Extract value from header using the value extractor.
fn extract_value(value: &HeaderValue, extractor: &ValueExtractor) -> Option<String> {
    let value_str = value.to_str().ok()?;

    match extractor {
        ValueExtractor::Full => Some(value_str.to_string()),
        ValueExtractor::Regex(regex) => {
            regex.captures(value_str).and_then(|caps| {
                // Return first capture group if exists, otherwise full match
                caps.get(1)
                    .or_else(|| caps.get(0))
                    .map(|m| m.as_str().to_string())
            })
        }
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for Header<E>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject, ctx: &mut EvalContext) -> KeyParts<Self::Subject> {
        let headers = &subject.parts().headers;
        let mut extracted_parts = Vec::new();

        match &self.name_selector {
            NameSelector::Exact(name) => {
                let value = headers
                    .get(name.as_str())
                    .and_then(|v| extract_value(v, &self.value_extractor))
                    .map(|v| apply_transform_chain(v, &self.transforms));

                extracted_parts.push(KeyPart::new(name.clone(), value));
            }
            NameSelector::Starts(prefix) => {
                for (name, value) in headers.iter() {
                    let name_str = name.as_str();
                    if name_str.starts_with(prefix.as_str()) {
                        let extracted = extract_value(value, &self.value_extractor)
                            .map(|v| apply_transform_chain(v, &self.transforms));

                        extracted_parts.push(KeyPart::new(name_str, extracted));
                    }
                }
                // Sort by header name for deterministic cache keys
                extracted_parts.sort_by(|a, b| a.key().cmp(b.key()));
            }
        }

        let mut parts = self.inner.get(subject, ctx).await;
        parts.append(&mut extracted_parts);
        parts
    }
}
