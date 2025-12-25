//! Query string parsing utilities.
//!
//! Provides [`parse`] for deserializing URL query strings into key-value maps.
//! Used internally by query predicates and extractors.
//!
//! # Supported Syntax
//!
//! - Simple parameters: `key=value`
//! - Multiple parameters: `key1=value1&key2=value2`
//! - Array syntax: `color[]=red&color[]=blue`
//! - Nested parameters: `filter[status]=active` (up to 5 levels deep)
//!
//! # Security
//!
//! Nesting depth is limited to 5 levels to prevent DoS attacks via deeply
//! nested queries that could cause stack overflow or memory exhaustion.
//!
//! # Examples
//!
//! ```
//! use hitbox_http::query::{parse, Value};
//!
//! let params = parse("page=1&limit=10").unwrap();
//! assert_eq!(params.get("page").unwrap().inner(), vec!["1"]);
//!
//! // Array parameters
//! let params = parse("color[]=red&color[]=blue").unwrap();
//! assert_eq!(params.get("color").unwrap().inner(), vec!["red", "blue"]);
//! ```

use serde::Deserialize;
use std::collections::HashMap;

/// Maximum nesting depth for query string parsing.
///
/// Limits structures like `a[b][c][d][e][f]=value` to prevent DoS attacks
/// via deeply nested queries that could cause stack overflow or memory exhaustion.
const MAX_QUERY_DEPTH: usize = 5;

/// A query parameter value, either a single string or an array of strings.
///
/// # Examples
///
/// ```
/// use hitbox_http::query::{parse, Value};
///
/// // Scalar value
/// let params = parse("name=alice").unwrap();
/// assert!(matches!(params.get("name"), Some(Value::Scalar(_))));
///
/// // Array value (using bracket syntax)
/// let params = parse("tags[]=rust&tags[]=http").unwrap();
/// assert!(matches!(params.get("tags"), Some(Value::Array(_))));
/// ```
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Value {
    /// A single parameter value.
    Scalar(String),
    /// Multiple values for the same parameter (array syntax).
    Array(Vec<String>),
}

impl Value {
    /// Returns all values as a vector.
    ///
    /// For [`Scalar`](Self::Scalar), returns a single-element vector.
    /// For [`Array`](Self::Array), returns all elements.
    pub fn inner(&self) -> Vec<String> {
        match self {
            Value::Scalar(value) => vec![value.to_owned()],
            Value::Array(values) => values.to_owned(),
        }
    }

    /// Returns `true` if the value contains the given string.
    pub fn contains(&self, value: &String) -> bool {
        self.inner().contains(value)
    }
}

/// Parses a query string into a map of key-value pairs.
///
/// Returns `None` if:
/// - The query string is malformed
/// - Nesting depth exceeds 5 levels
///
/// # Examples
///
/// ```
/// use hitbox_http::query::parse;
///
/// let params = parse("page=1&sort=name").unwrap();
/// assert_eq!(params.get("page").unwrap().inner(), vec!["1"]);
/// assert_eq!(params.get("sort").unwrap().inner(), vec!["name"]);
///
/// // Exceeds depth limit
/// assert!(parse("a[b][c][d][e][f][g]=1").is_none());
/// ```
pub fn parse(value: &str) -> Option<HashMap<String, Value>> {
    serde_qs::Config::new(MAX_QUERY_DEPTH, false)
        .deserialize_str(value)
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_one() {
        let hash_map = parse("key=value").unwrap();
        let value = hash_map.get("key").unwrap();
        assert_eq!(value.inner(), vec!["value"]);
    }

    #[test]
    fn test_parse_valid_multiple() {
        let hash_map = parse("key-one=value-one&key-two=value-two&key-three=value-three").unwrap();
        let value = hash_map.get("key-one").unwrap();
        assert_eq!(value.inner(), vec!["value-one"]);
        let value = hash_map.get("key-two").unwrap();
        assert_eq!(value.inner(), vec!["value-two"]);
        let value = hash_map.get("key-three").unwrap();
        assert_eq!(value.inner(), vec!["value-three"]);
    }

    #[test]
    fn test_parse_not_valid() {
        let hash_map = parse("   wrong   ").unwrap();
        assert_eq!(hash_map.len(), 1);
    }

    #[test]
    fn test_parse_exceeds_depth_returns_none() {
        // Nesting depth exceeds configured limit (5), should return None
        assert!(parse("a[b][c][d][e][f][g]=1").is_none());
    }

    #[test]
    fn test_parse_array_bracket_syntax() {
        // Note: serde_qs only supports bracket syntax for arrays (color[]=a&color[]=b)
        // Repeated keys without brackets (color=a&color=b) are not supported
        let hash_map = parse("color[]=red&color[]=blue&color[]=green").unwrap();
        let value = hash_map.get("color").unwrap();
        assert_eq!(value.inner(), vec!["red", "blue", "green"]);
    }
}
