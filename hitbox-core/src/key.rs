//! Cache key types and construction.
//!
//! This module provides types for building and representing cache keys:
//!
//! - [`CacheKey`] - The complete cache key with prefix, version, and parts
//! - [`KeyPart`] - A single key-value component of a cache key
//! - [`KeyParts`] - Builder for accumulating key parts during extraction
//!
//! ## Key Structure
//!
//! Cache keys have three components:
//!
//! 1. **Prefix** - Optional namespace for grouping related keys
//! 2. **Version** - Numeric version for cache invalidation
//! 3. **Parts** - List of key-value pairs extracted from requests
//!
//! ## Format
//!
//! When serialized to string, keys follow this format:
//! `{prefix}:v{version}:key1=value1&key2=value2`
//!
//! - Prefix is omitted if empty
//! - Version is omitted if zero
//!
//! ```
//! use hitbox_core::{CacheKey, KeyPart};
//!
//! // Full format: prefix + version + parts
//! let key = CacheKey::new("api", 1, vec![KeyPart::new("id", Some("42"))]);
//! assert_eq!(format!("{}", key), "api:v1:id=42");
//!
//! // No prefix
//! let key = CacheKey::new("", 2, vec![KeyPart::new("id", Some("42"))]);
//! assert_eq!(format!("{}", key), "v2:id=42");
//!
//! // No version (v0)
//! let key = CacheKey::new("cache", 0, vec![KeyPart::new("id", Some("42"))]);
//! assert_eq!(format!("{}", key), "cache:id=42");
//!
//! // No prefix, no version
//! let key = CacheKey::new("", 0, vec![KeyPart::new("id", Some("42"))]);
//! assert_eq!(format!("{}", key), "id=42");
//!
//! // KeyPart with None value (key only, no value)
//! let key = CacheKey::new("", 0, vec![KeyPart::new("flag", None::<&str>)]);
//! assert_eq!(format!("{}", key), "flag");
//!
//! // Mixed: Some and None values
//! let key = CacheKey::new("api", 1, vec![
//!     KeyPart::new("method", Some("GET")),
//!     KeyPart::new("cached", None::<&str>),
//! ]);
//! assert_eq!(format!("{}", key), "api:v1:method=GET&cached");
//! ```
//!
//! ## Performance
//!
//! [`CacheKey`] uses `Arc` internally for cheap cloning - copying a key
//! only increments a reference count rather than cloning all parts.
//!
//! [`KeyPart`] uses [`SmolStr`] for small string optimization - short
//! strings (≤23 bytes) are stored inline without heap allocation.

use bitcode::__private::{
    Buffer, Decoder, Encoder, Result, VariantDecoder, VariantEncoder, Vec, View,
};
use bitcode::{Decode, Encode};
use core::num::NonZeroUsize;
use smol_str::SmolStr;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Inner structure containing the actual cache key data.
/// Wrapped in Arc for cheap cloning.
#[derive(Debug, Eq, PartialEq, Hash, serde::Serialize)]
struct CacheKeyInner {
    parts: Vec<KeyPart>,
    version: u32,
    prefix: SmolStr,
    /// Precalculated size of heap-allocated string content.
    /// Only counts strings >23 bytes (SmolStr's inline threshold).
    #[serde(skip)]
    content_size: usize,
}

/// A cache key identifying a cached entry.
///
/// Cache keys are composed of:
/// - A **prefix** for namespacing (e.g., "api", "users")
/// - A **version** number for cache invalidation
/// - A list of **parts** (key-value pairs) extracted from requests
///
/// # Cheap Cloning
///
/// `CacheKey` wraps its data in [`Arc`], making `clone()` an O(1) operation
/// that only increments a reference count. This is important because keys
/// are frequently passed around during cache operations.
///
/// # Example
///
/// ```
/// use hitbox_core::{CacheKey, KeyPart};
///
/// let key = CacheKey::new(
///     "api",
///     1,
///     vec![
///         KeyPart::new("method", Some("GET")),
///         KeyPart::new("path", Some("/users/123")),
///     ],
/// );
///
/// assert_eq!(key.prefix(), "api");
/// assert_eq!(key.version(), 1);
/// assert_eq!(format!("{}", key), "api:v1:method=GET&path=/users/123");
/// ```
#[derive(Clone, Debug, serde::Serialize)]
#[serde(into = "CacheKeyInner")]
pub struct CacheKey {
    inner: Arc<CacheKeyInner>,
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        // Fast path: same Arc pointer
        Arc::ptr_eq(&self.inner, &other.inner) || self.inner == other.inner
    }
}

impl Eq for CacheKey {}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl From<CacheKey> for CacheKeyInner {
    fn from(key: CacheKey) -> Self {
        // Try to unwrap Arc, or clone if shared
        Arc::try_unwrap(key.inner).unwrap_or_else(|arc| (*arc).clone())
    }
}

impl Clone for CacheKeyInner {
    fn clone(&self) -> Self {
        CacheKeyInner {
            parts: self.parts.clone(),
            version: self.version,
            prefix: self.prefix.clone(),
            content_size: self.content_size,
        }
    }
}

impl CacheKeyInner {
    /// Calculate the size of heap-allocated string content.
    ///
    /// SmolStr stores strings ≤23 bytes inline (already counted in struct size).
    /// Only strings >23 bytes allocate on heap and need additional counting.
    fn calculate_content_size(prefix: &SmolStr, parts: &[KeyPart]) -> usize {
        let heap_size = |len: usize| len.saturating_sub(23);

        heap_size(prefix.len())
            + parts
                .iter()
                .map(|p| heap_size(p.key().len()) + p.value().map_or(0, |v| heap_size(v.len())))
                .sum::<usize>()
    }
}

impl fmt::Display for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Compact format: prefix:v{version}:key=value&key2=value2
        if !self.inner.prefix.is_empty() {
            write!(f, "{}:", self.inner.prefix)?;
        }
        if self.inner.version > 0 {
            write!(f, "v{}:", self.inner.version)?;
        }
        for (i, part) in self.inner.parts.iter().enumerate() {
            if i > 0 {
                write!(f, "&")?;
            }
            write!(f, "{}", part)?;
        }
        Ok(())
    }
}

impl Encode for CacheKey {
    type Encoder = CacheKeyEncoder;
}

impl<'de> Decode<'de> for CacheKey {
    type Decoder = CacheKeyDecoder<'de>;
}

#[doc(hidden)]
#[derive(Default)]
pub struct CacheKeyEncoder {
    parts: <Vec<KeyPart> as Encode>::Encoder,
    version: <u32 as Encode>::Encoder,
    prefix: <str as Encode>::Encoder,
}

impl Encoder<CacheKey> for CacheKeyEncoder {
    fn encode(&mut self, value: &CacheKey) {
        self.parts.encode(&value.inner.parts);
        self.version.encode(&value.inner.version);
        self.prefix.encode(value.inner.prefix.as_str());
    }
}

impl Buffer for CacheKeyEncoder {
    fn collect_into(&mut self, out: &mut Vec<u8>) {
        self.parts.collect_into(out);
        self.version.collect_into(out);
        self.prefix.collect_into(out);
    }

    fn reserve(&mut self, additional: NonZeroUsize) {
        self.parts.reserve(additional);
        self.version.reserve(additional);
        self.prefix.reserve(additional);
    }
}

#[doc(hidden)]
#[derive(Default)]
pub struct CacheKeyDecoder<'de> {
    parts: <Vec<KeyPart> as Decode<'de>>::Decoder,
    version: <u32 as Decode<'de>>::Decoder,
    prefix: <&'de str as Decode<'de>>::Decoder,
}

impl<'de> View<'de> for CacheKeyDecoder<'de> {
    fn populate(&mut self, input: &mut &'de [u8], length: usize) -> Result<()> {
        self.parts.populate(input, length)?;
        self.version.populate(input, length)?;
        self.prefix.populate(input, length)?;
        Ok(())
    }
}

impl<'de> Decoder<'de, CacheKey> for CacheKeyDecoder<'de> {
    fn decode(&mut self) -> CacheKey {
        let prefix_str: &str = self.prefix.decode();
        let prefix = SmolStr::new(prefix_str);
        let parts: Vec<KeyPart> = self.parts.decode();
        let content_size = CacheKeyInner::calculate_content_size(&prefix, &parts);
        CacheKey {
            inner: Arc::new(CacheKeyInner {
                parts,
                version: self.version.decode(),
                prefix,
                content_size,
            }),
        }
    }
}

impl CacheKey {
    /// Returns an iterator over the key parts.
    pub fn parts(&self) -> impl Iterator<Item = &KeyPart> {
        self.inner.parts.iter()
    }

    /// Returns the cache key version number.
    pub fn version(&self) -> u32 {
        self.inner.version
    }

    /// Returns the cache key prefix.
    pub fn prefix(&self) -> &str {
        &self.inner.prefix
    }

    /// Returns the estimated memory usage of this cache key in bytes.
    ///
    /// This includes:
    /// - Arc heap allocation (control block + CacheKeyInner)
    /// - Vec heap allocation (KeyPart elements)
    /// - SmolStr heap allocations (strings >23 bytes)
    pub fn memory_size(&self) -> usize {
        use std::mem::size_of;

        // Arc heap allocation: strong count + weak count + data
        let arc_overhead = 2 * size_of::<usize>() + size_of::<CacheKeyInner>();

        // Vec heap allocation: each KeyPart element
        let vec_overhead = self.inner.parts.len() * size_of::<KeyPart>();

        // SmolStr heap allocations: only strings >23 bytes
        let heap_strings = self.inner.content_size;

        arc_overhead + vec_overhead + heap_strings
    }

    /// Creates a new cache key with the given components.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Namespace prefix for the key
    /// * `version` - Version number for cache invalidation
    /// * `parts` - List of key-value parts
    pub fn new(prefix: impl Into<SmolStr>, version: u32, parts: Vec<KeyPart>) -> Self {
        let prefix = prefix.into();
        let content_size = CacheKeyInner::calculate_content_size(&prefix, &parts);
        CacheKey {
            inner: Arc::new(CacheKeyInner {
                parts,
                version,
                prefix,
                content_size,
            }),
        }
    }

    /// Creates a simple cache key with a single key-value part.
    ///
    /// The prefix is empty and version is 0.
    pub fn from_str(key: &str, value: &str) -> Self {
        let prefix = SmolStr::default();
        let parts = vec![KeyPart::new(key, Some(value))];
        let content_size = CacheKeyInner::calculate_content_size(&prefix, &parts);
        CacheKey {
            inner: Arc::new(CacheKeyInner {
                parts,
                version: 0,
                prefix,
                content_size,
            }),
        }
    }

    /// Creates a cache key from a slice of key-value pairs.
    ///
    /// The prefix is empty and version is 0.
    pub fn from_slice(parts: &[(&str, Option<&str>)]) -> Self {
        let prefix = SmolStr::default();
        let parts: Vec<KeyPart> = parts
            .iter()
            .map(|(key, value)| KeyPart::new(key, *value))
            .collect();
        let content_size = CacheKeyInner::calculate_content_size(&prefix, &parts);
        CacheKey {
            inner: Arc::new(CacheKeyInner {
                parts,
                version: 0,
                prefix,
                content_size,
            }),
        }
    }
}

/// A single component of a cache key.
///
/// Each part represents a key-value pair extracted from a request.
/// The value is optional - some parts may be key-only (flags).
///
/// # String Optimization
///
/// Both key and value use [`SmolStr`] which stores small strings (≤23 bytes)
/// inline without heap allocation. This is efficient for typical cache key
/// components like "method", "path", "GET", etc.
///
/// # Example
///
/// ```
/// use hitbox_core::KeyPart;
///
/// // Key with value
/// let method = KeyPart::new("method", Some("GET"));
/// assert_eq!(method.key(), "method");
/// assert_eq!(method.value(), Some("GET"));
///
/// // Key without value (flag)
/// let flag = KeyPart::new("cached", None::<&str>);
/// assert_eq!(flag.key(), "cached");
/// assert_eq!(flag.value(), None);
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct KeyPart {
    key: SmolStr,
    value: Option<SmolStr>,
}

impl fmt::Display for KeyPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.key)?;
        if let Some(ref value) = self.value {
            write!(f, "={}", value)?;
        }
        Ok(())
    }
}

impl Encode for KeyPart {
    type Encoder = KeyPartEncoder;
}

impl<'de> Decode<'de> for KeyPart {
    type Decoder = KeyPartDecoder<'de>;
}

#[doc(hidden)]
#[derive(Default)]
pub struct KeyPartEncoder {
    key: <str as Encode>::Encoder,
    // Manual Option encoding: variant (0=None, 1=Some) + value
    value_variant: VariantEncoder<2>,
    value_str: <str as Encode>::Encoder,
}

impl Encoder<KeyPart> for KeyPartEncoder {
    fn encode(&mut self, value: &KeyPart) {
        self.key.encode(value.key.as_str());
        // Manually encode Option<SmolStr> as variant + str
        self.value_variant.encode(&(value.value.is_some() as u8));
        if let Some(ref v) = value.value {
            self.value_str.reserve(NonZeroUsize::MIN);
            self.value_str.encode(v.as_str());
        }
    }
}

impl Buffer for KeyPartEncoder {
    fn collect_into(&mut self, out: &mut Vec<u8>) {
        self.key.collect_into(out);
        self.value_variant.collect_into(out);
        self.value_str.collect_into(out);
    }

    fn reserve(&mut self, additional: NonZeroUsize) {
        self.key.reserve(additional);
        self.value_variant.reserve(additional);
        // Don't reserve for value_str - we don't know how many are Some
    }
}

#[doc(hidden)]
#[derive(Default)]
pub struct KeyPartDecoder<'de> {
    key: <&'de str as Decode<'de>>::Decoder,
    value_variant: VariantDecoder<'de, 2, false>,
    value_str: <&'de str as Decode<'de>>::Decoder,
}

impl<'de> View<'de> for KeyPartDecoder<'de> {
    fn populate(&mut self, input: &mut &'de [u8], length: usize) -> Result<()> {
        self.key.populate(input, length)?;
        self.value_variant.populate(input, length)?;
        // Get the count of Some values from variant decoder
        let some_count = self.value_variant.length(1);
        self.value_str.populate(input, some_count)?;
        Ok(())
    }
}

impl<'de> Decoder<'de, KeyPart> for KeyPartDecoder<'de> {
    fn decode(&mut self) -> KeyPart {
        let key_str: &str = self.key.decode();
        let value = if self.value_variant.decode() != 0 {
            let value_str: &str = self.value_str.decode();
            Some(SmolStr::new(value_str))
        } else {
            None
        };
        KeyPart {
            key: SmolStr::new(key_str),
            value,
        }
    }
}

impl KeyPart {
    /// Creates a new key part.
    ///
    /// # Arguments
    ///
    /// * `key` - The key name
    /// * `value` - Optional value associated with the key
    pub fn new<K: AsRef<str>, V: AsRef<str>>(key: K, value: Option<V>) -> Self {
        KeyPart {
            key: SmolStr::new(key),
            value: value.map(SmolStr::new),
        }
    }

    /// Returns the key name.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the optional value.
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    /// Returns a new `KeyPart` with the key replaced, keeping the value.
    pub fn with_key(self, key: impl AsRef<str>) -> Self {
        KeyPart {
            key: SmolStr::new(key),
            ..self
        }
    }

    /// Returns a new `KeyPart` with the key prefixed by `prefix.`.
    ///
    /// For example, `KeyPart("id", "42").prefixed("user")` produces `KeyPart("user.id", "42")`.
    pub fn prefixed(self, prefix: impl AsRef<str>) -> Self {
        KeyPart {
            key: SmolStr::new(format!("{}.{}", prefix.as_ref(), self.key)),
            ..self
        }
    }
}

/// Builder for accumulating cache key parts during extraction.
///
/// `KeyParts` carries both the subject being processed and the accumulated
/// key parts. This allows extractors to be chained while building up the
/// complete cache key.
///
/// # Type Parameter
///
/// * `T` - The subject type (usually a request type)
///
/// # Usage
///
/// Extractors receive a `KeyParts<T>`, add their parts, and return it for
/// the next extractor in the chain. Finally, `into_cache_key()` is called
/// to produce the final [`CacheKey`].
#[derive(Debug)]
pub struct KeyParts<T: Sized> {
    subject: T,
    parts: Vec<KeyPart>,
}

impl<T> KeyParts<T> {
    /// Creates a new `KeyParts` wrapping the given subject.
    pub fn new(subject: T) -> Self {
        KeyParts {
            subject,
            parts: Vec::new(),
        }
    }

    /// Adds a single key part.
    pub fn push(&mut self, part: KeyPart) {
        self.parts.push(part)
    }

    /// Appends multiple key parts from a vector.
    pub fn append(&mut self, parts: &mut Vec<KeyPart>) {
        self.parts.append(parts)
    }

    /// Consumes the builder and returns the subject with its cache key.
    ///
    /// The returned cache key has an empty prefix and version 0.
    pub fn into_cache_key(self) -> (T, CacheKey) {
        let prefix = SmolStr::default();
        let content_size = CacheKeyInner::calculate_content_size(&prefix, &self.parts);
        (
            self.subject,
            CacheKey {
                inner: Arc::new(CacheKeyInner {
                    version: 0,
                    prefix,
                    parts: self.parts,
                    content_size,
                }),
            },
        )
    }
}
