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
//!
//! // Hash and truncate to 16 hex characters
//! let transforms = vec![Transform::Hash, Transform::Truncate(16)];
//! ```

use sha2::{Digest, Sha256};

/// Transforms extracted values before they become cache key parts.
///
/// Multiple transforms can be chained and are applied in order.
#[derive(Debug, Clone, Copy)]
pub enum Transform {
    /// Full SHA256 hash (64 hex characters).
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
    /// Truncate to the given number of characters.
    ///
    /// Useful after hashing to shorten cache keys when full collision
    /// resistance is not needed. For example, `Hash` + `Truncate(16)`
    /// gives a 16-character hex digest.
    Truncate(usize),
}

/// Apply SHA256 hash to value (full 64 hex characters).
pub fn apply_hash(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Apply a single transform to a value.
pub fn apply_single_transform(value: String, transform: &Transform) -> String {
    match transform {
        Transform::Hash => apply_hash(&value),
        Transform::Lowercase => value.to_lowercase(),
        Transform::Uppercase => value.to_uppercase(),
        Transform::Truncate(len) => {
            let mut s = value;
            s.truncate(*len);
            s
        }
    }
}

/// Apply a chain of transforms to a value.
pub fn apply_transform_chain(mut value: String, chain: &[Transform]) -> String {
    for transform in chain {
        value = apply_single_transform(value, transform);
    }
    value
}
