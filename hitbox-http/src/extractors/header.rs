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
//! use hitbox_http::extractors::{Method, header::HeaderExtractor};
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! # use hitbox_http::extractors::{NeutralExtractor, header::Header};
//! let extractor = Method::new()
//!     .header("x-api-key".to_string());
//! # let _: &Header<Method<NeutralExtractor<Empty<Bytes>>>> = &extractor;
//! ```

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};
use http::HeaderValue;
use regex::Regex;

use super::NeutralExtractor;
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

/// Extracts values from header content.
#[derive(Debug, Clone)]
pub enum ValueExtractor {
    /// Use the full header value.
    Full,
    /// Extract using regex (returns first capture group, or full match if no groups).
    Regex(Regex),
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

impl<S> Header<NeutralExtractor<S>> {
    /// Creates a header extractor for a single header by exact name.
    ///
    /// The header value becomes a cache key part with the header name
    /// as key. For more complex extraction (prefix matching, regex, transforms),
    /// use [`Header::new_with`].
    ///
    /// Chain onto existing extractors using [`HeaderExtractor::header`] instead
    /// if you already have an extractor chain.
    pub fn new(name: String) -> Self {
        Self {
            inner: NeutralExtractor::new(),
            name_selector: NameSelector::Exact(name),
            value_extractor: ValueExtractor::Full,
            transforms: Vec::new(),
        }
    }
}

impl<E> Header<E> {
    /// Creates a header extractor with full configuration options.
    ///
    /// This constructor provides complete control over header extraction:
    /// - Select headers by exact name or prefix pattern
    /// - Extract full values or use regex capture groups
    /// - Apply transformations (hash, lowercase, uppercase)
    ///
    /// For simple exact-name extraction without transforms, use [`Header::new`]
    /// or [`HeaderExtractor::header`] instead.
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
    /// Adds extraction for a single header by exact name.
    fn header(self, name: String) -> Header<Self>;
}

impl<E> HeaderExtractor for E
where
    E: Extractor,
{
    fn header(self, name: String) -> Header<Self> {
        Header {
            inner: self,
            name_selector: NameSelector::Exact(name),
            value_extractor: ValueExtractor::Full,
            transforms: Vec::new(),
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

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
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

        let mut parts = self.inner.get(subject).await;
        parts.append(&mut extracted_parts);
        parts
    }
}
