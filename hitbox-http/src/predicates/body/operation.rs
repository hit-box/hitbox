use std::fmt::Debug;

use hitbox::predicate::PredicateResult;
use hyper::body::Body as HttpBody;

use super::{
    jq::{JqExpression, JqOperation},
    plain::PlainOperation,
};
use crate::BufferedBody;

/// Matching operations for HTTP body content.
///
/// # Caveats
///
/// Body predicates may consume bytes from the stream. The body is transitioned
/// to [`BufferedBody::Partial`] or [`BufferedBody::Complete`] after evaluation.
///
/// # Examples
///
/// ## Size Limit
///
/// Only cache responses smaller than 1MB:
///
/// ```
/// use hitbox_http::predicates::body::Operation;
///
/// let op = Operation::Limit { bytes: 1024 * 1024 };
/// ```
///
/// ## Plain Text Matching
///
/// Cache only if body contains a success marker:
///
/// ```
/// use bytes::Bytes;
/// use hitbox_http::predicates::body::{Operation, PlainOperation};
///
/// let op = Operation::Plain(PlainOperation::Contains(Bytes::from("\"success\":true")));
/// ```
///
/// Cache only if body starts with JSON array:
///
/// ```
/// use bytes::Bytes;
/// use hitbox_http::predicates::body::{Operation, PlainOperation};
///
/// let op = Operation::Plain(PlainOperation::Starts(Bytes::from("[")));
/// ```
///
/// Cache only if body matches a regex pattern:
///
/// ```
/// use hitbox_http::predicates::body::{Operation, PlainOperation};
///
/// let regex = regex::bytes::Regex::new(r#""status":\s*"(ok|success)""#).unwrap();
/// let op = Operation::Plain(PlainOperation::RegExp(regex));
/// ```
///
/// ## JQ (JSON) Matching
///
/// Cache only if response has non-empty items array:
///
/// ```
/// use hitbox_http::predicates::body::{Operation, JqExpression, JqOperation};
///
/// let op = Operation::Jq {
///     filter: JqExpression::compile(".items | length > 0").unwrap(),
///     operation: JqOperation::Eq(serde_json::Value::Bool(true)),
/// };
/// ```
///
/// Cache only if user role exists:
///
/// ```
/// use hitbox_http::predicates::body::{Operation, JqExpression, JqOperation};
///
/// let op = Operation::Jq {
///     filter: JqExpression::compile(".user.role").unwrap(),
///     operation: JqOperation::Exist,
/// };
/// ```
///
/// Cache only if status is one of allowed values:
///
/// ```
/// use hitbox_http::predicates::body::{Operation, JqExpression, JqOperation};
///
/// let op = Operation::Jq {
///     filter: JqExpression::compile(".status").unwrap(),
///     operation: JqOperation::In(vec![
///         serde_json::json!("published"),
///         serde_json::json!("approved"),
///     ]),
/// };
/// ```
#[derive(Debug)]
pub enum Operation {
    /// Use when you need to limit cached response sizes.
    ///
    /// Best for preventing cache bloat from large responses like file downloads.
    /// Reads up to `bytes + 1` to determine if the limit is exceeded.
    Limit {
        /// Maximum body size in bytes.
        bytes: usize,
    },
    /// Use when matching raw body bytes without parsing.
    ///
    /// Best for text responses, checking signatures, or simple content matching.
    Plain(PlainOperation),
    /// Use when caching depends on JSON content structure or values.
    ///
    /// Best for APIs where cacheability depends on response data (e.g., user roles, feature flags).
    Jq {
        /// The compiled JQ filter expression.
        filter: JqExpression,
        /// The operation to apply to the filter result.
        operation: JqOperation,
    },
}

impl Operation {
    /// Check if the operation matches the body.
    /// Returns `PredicateResult::Cacheable` if the operation is satisfied,
    /// `PredicateResult::NonCacheable` otherwise.
    pub async fn check<B>(&self, body: BufferedBody<B>) -> PredicateResult<BufferedBody<B>>
    where
        B: HttpBody + Unpin,
        B::Data: Send,
        B::Error: Debug,
    {
        match self {
            Operation::Limit { bytes } => {
                use crate::CollectExactResult;

                // Check size hint first for optimization
                if let Some(upper) = body.size_hint().upper()
                    && upper > *bytes as u64
                {
                    // Size hint indicates body exceeds limit - non-cacheable
                    return PredicateResult::NonCacheable(body);
                }

                // Try to read limit+1 bytes to check if body exceeds limit
                let result = body.collect_exact(*bytes + 1).await;

                match result {
                    CollectExactResult::AtLeast { .. } => {
                        // Got limit+1 bytes, so body exceeds limit
                        PredicateResult::NonCacheable(result.into_buffered_body())
                    }
                    CollectExactResult::Incomplete { ref error, .. } => {
                        let is_error = error.is_some();
                        let body = result.into_buffered_body();
                        if is_error {
                            // Error occurred - non-cacheable
                            PredicateResult::NonCacheable(body)
                        } else {
                            // Within limit, no error - cacheable
                            PredicateResult::Cacheable(body)
                        }
                    }
                }
            }
            Operation::Plain(plain_op) => plain_op.check(body).await,
            Operation::Jq { filter, operation } => operation.check(filter, body).await,
        }
    }
}
