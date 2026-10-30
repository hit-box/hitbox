//! Shared value transformation configuration.

use hitbox_http::extractors::transform::Transform as HttpTransform;
use serde::{Deserialize, Serialize};

/// Value transformation.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Transform {
    /// Full SHA256 hash (64 hex chars)
    Hash,
    /// Convert to lowercase
    Lowercase,
    /// Convert to uppercase
    Uppercase,
    /// Truncate to the given number of characters
    Truncate(usize),
}

impl Transform {
    /// Convert to HTTP extractor transform.
    pub fn into_http(self) -> HttpTransform {
        match self {
            Transform::Hash => HttpTransform::Hash,
            Transform::Lowercase => HttpTransform::Lowercase,
            Transform::Uppercase => HttpTransform::Uppercase,
            Transform::Truncate(n) => HttpTransform::Truncate(n),
        }
    }
}

/// Convert a vector of transforms to HTTP transforms.
pub fn into_http_transforms(transforms: Vec<Transform>) -> Vec<HttpTransform> {
    transforms.into_iter().map(Transform::into_http).collect()
}
