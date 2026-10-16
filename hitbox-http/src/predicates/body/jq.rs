//! JQ expression support for body predicates.
//!
//! Re-exports [`JqExpression`] for compiling jq filters, and provides
//! [`JqOperation`] for matching against jq query results.

use hitbox::predicate::PredicateResult;
use http::Extensions;
use hyper::body::Body as HttpBody;
use serde_json::Value;

use crate::BufferedBody;
pub use crate::jq::JqExpression;
use crate::jq::ParsedBody;

/// Operations for matching jq query results.
///
/// Used with [`JqExpression`] to check if a JSON body matches certain criteria.
///
/// # Variants
///
/// - [`Eq`](Self::Eq) — The jq result must equal the specified value
/// - [`Exist`](Self::Exist) — The jq result must be non-null
/// - [`In`](Self::In) — The jq result must be one of the specified values
#[derive(Debug, Clone)]
pub enum JqOperation {
    /// Match if the jq result equals this value.
    Eq(Value),
    /// Match if the jq result is non-null (path exists).
    Exist,
    /// Match if the jq result is one of these values.
    In(Vec<Value>),
}

impl JqOperation {
    /// Checks if the jq operation matches the body.
    ///
    /// Collects the entire body, parses it as JSON, applies the jq filter,
    /// and checks if the result satisfies this operation.
    ///
    /// `extensions` are the subject's: the parsed body is kept there, so other
    /// jq predicates and extractors on the same subject reuse it.
    ///
    /// Returns [`Cacheable`](PredicateResult::Cacheable) if the operation is satisfied,
    /// [`NonCacheable`](PredicateResult::NonCacheable) otherwise.
    ///
    /// # Caveats
    ///
    /// - The entire body is buffered into memory for JSON parsing
    /// - Returns `NonCacheable` if the body is not valid JSON
    pub async fn check<B>(
        &self,
        filter: &JqExpression,
        body: BufferedBody<B>,
        extensions: &mut Extensions,
    ) -> PredicateResult<BufferedBody<B>>
    where
        B: HttpBody + Unpin,
        B::Data: Send,
    {
        // Collect the full body to parse as JSON
        let collected = match body.collect().await {
            Ok(c) => c,
            Err(error_body) => return PredicateResult::NonCacheable(error_body),
        };
        let (body_bytes, body_trailers) = (collected.data, collected.trailers);

        let found_value = ParsedBody::get_or_parse(extensions, &body_bytes)
            .and_then(|input| filter.apply_val(input));

        // Check if the operation matches; a body that is not JSON never does
        let matches = match self {
            JqOperation::Eq(expected) => {
                found_value.as_ref().map(|v| v == expected).unwrap_or(false)
            }
            JqOperation::Exist => found_value.is_some(),
            JqOperation::In(values) => found_value
                .as_ref()
                .map(|v| values.contains(v))
                .unwrap_or(false),
        };

        let result_body = BufferedBody::Complete {
            data: Some(body_bytes),
            trailers: body_trailers,
        };
        if matches {
            PredicateResult::Cacheable(result_body)
        } else {
            PredicateResult::NonCacheable(result_body)
        }
    }
}
