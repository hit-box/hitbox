use http::{HeaderMap, HeaderName, HeaderValue};
use regex::Regex;

/// Matching operations for HTTP headers.
///
/// These operations can be used with both request and response headers.
///
/// # Variants
///
/// - [`Eq`](Self::Eq): Exact match on header value
/// - [`Exist`](Self::Exist): Header presence check (any value)
/// - [`In`](Self::In): Match any of several values
/// - [`Contains`](Self::Contains): Substring match within header value
/// - [`Regex`](Self::Regex): Pattern match using regular expression
///
/// # Examples
///
/// ```
/// use hitbox_http::predicates::header::Operation;
/// use http::header::CONTENT_TYPE;
///
/// // Match exact header value
/// let op = Operation::Eq(
///     CONTENT_TYPE,
///     "application/json".parse().unwrap(),
/// );
///
/// // Check header exists
/// let op = Operation::Exist(CONTENT_TYPE);
///
/// // Match substring in header value
/// let op = Operation::Contains(CONTENT_TYPE, "json".to_string());
/// ```
///
/// Using regex for complex patterns:
///
/// ```
/// use hitbox_http::predicates::header::Operation;
/// use http::header::ACCEPT;
/// use regex::Regex;
///
/// // Match Accept headers containing version info
/// let op = Operation::Regex(
///     ACCEPT,
///     Regex::new(r"application/vnd\.api\+json; version=\d+").unwrap(),
/// );
/// ```
#[derive(Debug)]
pub enum Operation {
    /// Use when you need an exact match on a known header value.
    ///
    /// Best for specific content types, authorization schemes, or cache directives.
    Eq(HeaderName, HeaderValue),
    /// Use when presence of a header determines cacheability, regardless of value.
    ///
    /// Best for checking optional headers like `Authorization` or custom API headers.
    Exist(HeaderName),
    /// Use when any of several values should trigger caching.
    ///
    /// Best for allowing multiple content types or API versions.
    In(HeaderName, Vec<HeaderValue>),
    /// Use when matching partial values in complex headers.
    ///
    /// Best for content types with parameters (e.g., `"json"` in `application/json; charset=utf-8`).
    Contains(HeaderName, String),
    /// Use when header values follow a pattern.
    ///
    /// Best for version strings, custom formats, or extracting structured data.
    Regex(HeaderName, Regex),
}

impl Operation {
    /// Check if the operation matches the headers.
    pub fn check(&self, headers: &HeaderMap) -> bool {
        match self {
            Operation::Eq(name, value) => headers
                .get_all(name)
                .iter()
                .any(|header_value| value.eq(header_value)),
            Operation::Exist(name) => headers.get(name).is_some(),
            Operation::In(name, values) => headers
                .get_all(name)
                .iter()
                .any(|header_value| values.iter().any(|v| v.eq(header_value))),
            Operation::Contains(name, substring) => {
                headers.get_all(name).iter().any(|header_value| {
                    header_value
                        .to_str()
                        .map(|s| s.contains(substring.as_str()))
                        .unwrap_or(false)
                })
            }
            Operation::Regex(name, regex) => headers.get_all(name).iter().any(|header_value| {
                header_value
                    .to_str()
                    .map(|s| regex.is_match(s))
                    .unwrap_or(false)
            }),
        }
    }
}
