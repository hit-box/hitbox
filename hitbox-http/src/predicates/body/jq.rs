//! JQ expression support for body predicates.
//!
//! Provides [`JqExpression`] for compiling and applying jq filters to JSON bodies,
//! and [`JqOperation`] for matching against jq query results.

use std::fmt::Debug;

use hitbox::predicate::PredicateResult;
use hyper::body::Body as HttpBody;
use jaq_core::{
    Ctx, Filter, Native, RcIter,
    load::{Arena, File, Loader},
};
use jaq_json::{self, Val};
use serde_json::Value;

use crate::BufferedBody;

/// A compiled jq expression for querying JSON bodies.
///
/// Wraps a jaq filter that can be compiled once and reused for multiple requests.
/// This avoids the overhead of parsing and compiling the jq expression on each request.
///
/// # Examples
///
/// ```
/// use hitbox_http::predicates::body::JqExpression;
///
/// // Compile a jq expression
/// let expr = JqExpression::compile(".user.id").unwrap();
///
/// // Apply to JSON data
/// let json = serde_json::json!({"user": {"id": 42}});
/// let result = expr.apply(json);
/// assert_eq!(result, Some(serde_json::json!(42)));
/// ```
///
/// # Errors
///
/// [`compile`](Self::compile) returns `Err` if the jq expression is invalid.
#[derive(Clone)]
pub struct JqExpression(Filter<Native<Val>>);

impl JqExpression {
    /// Compiles a jq expression into a reusable filter.
    ///
    /// # Arguments
    ///
    /// * `expression` — A jq filter expression (e.g., `.user.id`, `.items[] | .name`)
    ///
    /// # Errors
    ///
    /// Returns `Err` if the expression cannot be parsed or compiled.
    pub fn compile(expression: &str) -> Result<Self, String> {
        let program = File {
            code: expression,
            path: (),
        };
        let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()));
        let arena = Arena::default();
        let modules = loader
            .load(&arena, program)
            .map_err(|e| format!("Failed to load jq program: {:?}", e))?;
        let filter = jaq_core::Compiler::default()
            .with_funs(jaq_std::funs().chain(jaq_json::funs()))
            .compile(modules)
            .map_err(|e| format!("Failed to compile jq program: {:?}", e))?;
        Ok(Self(filter))
    }

    /// Applies the filter to a JSON value and returns the result.
    ///
    /// Returns `None` if the filter produces `null` or no output.
    /// If the filter produces multiple values, they are returned as a JSON array.
    pub fn apply(&self, input: Value) -> Option<Value> {
        let inputs = RcIter::new(core::iter::empty());
        let out = self.0.run((Ctx::new([], &inputs), Val::from(input)));
        let results: Result<Vec<_>, _> = out.collect();
        match results {
            Ok(values) if values.eq(&vec![Val::Null]) => None,
            Ok(values) if !values.is_empty() => {
                let mut values: Vec<Value> = values.into_iter().map(|v| v.into()).collect();
                if values.len() == 1 {
                    values.pop()
                } else {
                    Some(Value::Array(values))
                }
            }
            _ => None,
        }
    }
}

impl Debug for JqExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JqExpression").finish_non_exhaustive()
    }
}

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
    ) -> PredicateResult<BufferedBody<B>>
    where
        B: HttpBody + Unpin,
        B::Data: Send,
        B::Error: Debug,
    {
        // Collect the full body to parse as JSON
        let body_bytes = match body.collect().await {
            Ok(bytes) => bytes,
            Err(error_body) => return PredicateResult::NonCacheable(error_body),
        };

        // Parse body as JSON
        let json_value: Value = match serde_json::from_slice(&body_bytes) {
            Ok(v) => v,
            Err(_) => {
                // Failed to parse JSON - non-cacheable
                return PredicateResult::NonCacheable(BufferedBody::Complete(Some(body_bytes)));
            }
        };

        // Apply the jq filter
        let found_value = filter.apply(json_value);

        // Check if the operation matches
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

        let result_body = BufferedBody::Complete(Some(body_bytes));
        if matches {
            PredicateResult::Cacheable(result_body)
        } else {
            PredicateResult::NonCacheable(result_body)
        }
    }
}
