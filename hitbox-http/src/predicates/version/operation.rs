//! HTTP version matching operations.

use http::Version;

/// Operations for matching HTTP versions.
///
/// # Examples
///
/// ```
/// use hitbox_http::predicates::version::Operation;
/// use http::Version;
///
/// // Match HTTP/2 only
/// let op = Operation::Eq(Version::HTTP_2);
/// assert!(op.check(Version::HTTP_2));
/// assert!(!op.check(Version::HTTP_11));
///
/// // Match HTTP/1.1 or HTTP/2
/// let op = Operation::In(vec![Version::HTTP_11, Version::HTTP_2]);
/// assert!(op.check(Version::HTTP_11));
/// assert!(op.check(Version::HTTP_2));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    /// Use when caching should apply to a specific HTTP version only.
    ///
    /// Best for HTTP/2-only features or HTTP/1.1 fallback scenarios.
    Eq(Version),
    /// Use when caching should apply to multiple HTTP versions.
    ///
    /// Best for supporting both HTTP/1.1 and HTTP/2 while excluding HTTP/1.0.
    In(Vec<Version>),
}

impl Operation {
    /// Check if the operation matches the given version
    pub fn check(&self, version: Version) -> bool {
        match self {
            Operation::Eq(expected) => version == *expected,
            Operation::In(versions) => versions.contains(&version),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq_matches() {
        let op = Operation::Eq(Version::HTTP_11);
        assert!(op.check(Version::HTTP_11));
        assert!(!op.check(Version::HTTP_2));
    }

    #[test]
    fn test_in_matches() {
        let op = Operation::In(vec![Version::HTTP_11, Version::HTTP_2]);
        assert!(op.check(Version::HTTP_11));
        assert!(op.check(Version::HTTP_2));
        assert!(!op.check(Version::HTTP_10));
    }
}
