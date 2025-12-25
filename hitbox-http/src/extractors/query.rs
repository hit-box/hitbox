//! Query parameter extraction for cache keys.
//!
//! Provides [`Query`] extractor with support for name selection, value extraction,
//! and transformation.
//!
//! # Examples
//!
//! Extract pagination parameters:
//!
//! ```
//! use hitbox_http::extractors::{Method, query::QueryExtractor};
//!
//! # use bytes::Bytes;
//! # use http_body_util::Empty;
//! # use hitbox_http::extractors::{NeutralExtractor, query::Query};
//! let extractor = Method::new()
//!     .query("page".to_string())
//!     .query("limit".to_string());
//! # let _: &Query<Query<Method<NeutralExtractor<Empty<Bytes>>>>> = &extractor;
//! ```

use async_trait::async_trait;
use hitbox::{Extractor, KeyPart, KeyParts};
use regex::Regex;

use super::NeutralExtractor;
pub use super::transform::Transform;
use super::transform::apply_transform_chain;
use crate::CacheableHttpRequest;

/// Selects which query parameters to extract.
#[derive(Debug, Clone)]
pub enum NameSelector {
    /// Match a single parameter by exact name.
    Exact(String),
    /// Match all parameters starting with a prefix.
    ///
    /// Results are sorted by parameter name for deterministic cache keys.
    Starts(String),
}

/// Extracts values from query parameter content.
#[derive(Debug, Clone)]
pub enum ValueExtractor {
    /// Use the full parameter value.
    Full,
    /// Extract using regex (returns first capture group, or full match if no groups).
    Regex(Regex),
}

/// Extracts query parameters as cache key parts.
///
/// Supports flexible parameter selection, value extraction, and transformation.
/// Array parameters (e.g., `color[]=red&color[]=blue`) are handled correctly.
///
/// # Key Parts Generated
///
/// For each matched parameter, generates a `KeyPart` with:
/// - Key: the parameter name
/// - Value: the extracted (and optionally transformed) value
///
/// # Performance
///
/// - Query string parsing allocates a `HashMap` for parameter lookup
/// - When using [`NameSelector::Starts`], results are sorted alphabetically
///   for deterministic cache keys (O(n log n) where n is matched parameters)
/// - Regex extraction ([`ValueExtractor::Regex`]) compiles the pattern once
///   at construction time
#[derive(Debug)]
pub struct Query<E> {
    inner: E,
    name_selector: NameSelector,
    value_extractor: ValueExtractor,
    transforms: Vec<Transform>,
}

impl<S> Query<NeutralExtractor<S>> {
    /// Creates a query extractor for a single parameter by exact name.
    ///
    /// The parameter value becomes a cache key part with the parameter name
    /// as key. For more complex extraction (prefix matching, regex, transforms),
    /// use [`Query::new_with`].
    ///
    /// Chain onto existing extractors using [`QueryExtractor::query`] instead
    /// if you already have an extractor chain.
    ///
    /// # Examples
    ///
    /// ```
    /// use hitbox_http::extractors::query::Query;
    ///
    /// # use bytes::Bytes;
    /// # use http_body_util::Empty;
    /// # use hitbox_http::extractors::NeutralExtractor;
    /// // Extract the "page" query parameter
    /// let extractor = Query::new("page".to_string());
    /// # let _: &Query<NeutralExtractor<Empty<Bytes>>> = &extractor;
    /// ```
    pub fn new(name: String) -> Self {
        Self {
            inner: NeutralExtractor::new(),
            name_selector: NameSelector::Exact(name),
            value_extractor: ValueExtractor::Full,
            transforms: Vec::new(),
        }
    }
}

impl<E> Query<E> {
    /// Creates a query parameter extractor with full configuration options.
    ///
    /// This constructor provides complete control over query extraction:
    /// - Select parameters by exact name or prefix pattern
    /// - Extract full values or use regex capture groups
    /// - Apply transformations (hash, lowercase, uppercase)
    ///
    /// For simple exact-name extraction without transforms, use [`Query::new`]
    /// or [`QueryExtractor::query`] instead.
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

/// Extension trait for adding query parameter extraction to an extractor chain.
///
/// # For Callers
///
/// Chain this to extract URL query parameters as cache key parts. Each
/// extracted parameter becomes a key part with the parameter name and value.
///
/// # For Implementors
///
/// This trait is automatically implemented for all [`Extractor`]
/// types. You don't need to implement it manually.
pub trait QueryExtractor: Sized {
    /// Adds extraction for a single query parameter by name.
    fn query(self, name: String) -> Query<Self>;
}

impl<E> QueryExtractor for E
where
    E: Extractor,
{
    fn query(self, name: String) -> Query<Self> {
        Query {
            inner: self,
            name_selector: NameSelector::Exact(name),
            value_extractor: ValueExtractor::Full,
            transforms: Vec::new(),
        }
    }
}

/// Extract value using the value extractor.
fn extract_value(value: &str, extractor: &ValueExtractor) -> Option<String> {
    match extractor {
        ValueExtractor::Full => Some(value.to_string()),
        ValueExtractor::Regex(regex) => regex
            .captures(value)
            .and_then(|caps| caps.get(1).or_else(|| caps.get(0)))
            .map(|m| m.as_str().to_string()),
    }
}

#[async_trait]
impl<ReqBody, E> Extractor for Query<E>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
{
    type Subject = E::Subject;

    async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
        let query_map = subject
            .parts()
            .uri
            .query()
            .and_then(crate::query::parse)
            .unwrap_or_default();

        let mut extracted_parts: Vec<KeyPart> = match &self.name_selector {
            NameSelector::Exact(name) => query_map
                .get(name)
                .map(|v| v.inner())
                .unwrap_or_default()
                .into_iter()
                .filter_map(|value| {
                    extract_value(&value, &self.value_extractor)
                        .map(|v| apply_transform_chain(v, &self.transforms))
                        .map(|v| KeyPart::new(name.clone(), Some(v)))
                })
                .collect(),

            NameSelector::Starts(prefix) => {
                let mut parts: Vec<KeyPart> = query_map
                    .iter()
                    .filter(|(name, _)| name.starts_with(prefix.as_str()))
                    .flat_map(|(name, value)| {
                        value.inner().into_iter().filter_map(|v| {
                            extract_value(&v, &self.value_extractor)
                                .map(|extracted| apply_transform_chain(extracted, &self.transforms))
                                .map(|extracted| KeyPart::new(name.clone(), Some(extracted)))
                        })
                    })
                    .collect();
                // Sort by parameter name for deterministic cache keys
                parts.sort_by(|a, b| a.key().cmp(b.key()));
                parts
            }
        };

        let mut parts = self.inner.get(subject).await;
        parts.append(&mut extracted_parts);
        parts
    }
}
