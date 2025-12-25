//! Shared value transformations for extractors.
//!
//! Transforms modify extracted values before they become part of the cache key.
//! They can be chained to apply multiple transformations in sequence.
//!
//! # Examples
//!
//! ```
//! use hitbox_http::extractors::transform::Transform;
//!
//! // Hash sensitive values to avoid storing them in cache keys
//! let transforms = vec![Transform::Hash];
//!
//! // Normalize case for case-insensitive matching
//! let transforms = vec![Transform::Lowercase];
//! ```

use sha2::{Digest, Sha256};

/// Transforms extracted values before they become cache key parts.
///
/// Multiple transforms can be chained and are applied in order.
#[derive(Debug, Clone, Copy)]
pub enum Transform {
    /// SHA256 hash, truncated to 16 hex characters.
    ///
    /// Useful for hashing sensitive values (API keys, tokens) to avoid
    /// storing them directly in cache keys while still differentiating requests.
    Hash,
    /// Convert to lowercase.
    ///
    /// Useful for case-insensitive cache key matching.
    Lowercase,
    /// Convert to uppercase.
    Uppercase,
}

/// Apply SHA256 hash to value (truncated to 16 hex chars).
pub fn apply_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8])
}

/// Apply a single transform to a value.
pub fn apply_single_transform(value: String, transform: &Transform) -> String {
    match transform {
        Transform::Hash => apply_hash(&value),
        Transform::Lowercase => value.to_lowercase(),
        Transform::Uppercase => value.to_uppercase(),
    }
}

/// Apply a chain of transforms to a value.
pub fn apply_transform_chain(mut value: String, chain: &[Transform]) -> String {
    for transform in chain {
        value = apply_single_transform(value, transform);
    }
    value
}
