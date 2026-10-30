//! jq expressions over JSON bodies.
//!
//! [`JqExpression`] compiles a jq filter once and runs it for body predicates
//! and extractors. Within one evaluation phase, every jq predicate and
//! extractor on the same request or response shares a single parse of the body.

use std::fmt::{self, Debug};
use std::str::FromStr;
use std::sync::Arc;

use http::Extensions;
use jaq_core::load::{Arena, File, Loader};
use jaq_core::native::{self, Fun, bome, v};
use jaq_core::{Compiler, Ctx, DataT, RunPtr, ValXs, Vars, data};
use jaq_json::{Num, Rc, Val, read};
use serde_json::{Number, Value};

use crate::extractors::transform::apply_hash;

/// Data the filters run on: plain values, no extra context.
type JqData = data::JustLut<Val>;

/// A compiled jq expression.
///
/// Compile once and reuse for every request: cloning is cheap, since clones
/// share the compiled filter. Besides the jq standard library, expressions can
/// call `hash`, which replaces a value with the hex SHA-256 of its string form.
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
pub struct JqExpression {
    expression: Arc<str>,
    filter: Arc<jaq_core::Filter<JqData>>,
}

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
        let defs = jaq_core::defs()
            .chain(jaq_std::defs())
            .chain(jaq_json::defs());
        let loader = Loader::new(defs);
        let arena = Arena::default();
        let modules = loader
            .load(&arena, program)
            .map_err(|e| format!("Failed to load jq program: {e:?}"))?;
        let funs = jaq_core::funs()
            .chain(jaq_std::funs())
            .chain(jaq_json::funs())
            .chain(hitbox_funs());
        let filter = Compiler::default()
            .with_funs(funs)
            .compile(modules)
            .map_err(|e| format!("Failed to compile jq program: {e:?}"))?;
        Ok(Self {
            expression: expression.into(),
            filter: Arc::new(filter),
        })
    }

    /// Returns the expression source this filter was compiled from.
    pub fn as_str(&self) -> &str {
        &self.expression
    }

    /// Applies the filter to a JSON value and returns the result.
    ///
    /// Returns `None` if the filter produces `null`, no output, or an error.
    /// If the filter produces multiple values, they are returned as a JSON array.
    pub fn apply(&self, input: Value) -> Option<Value> {
        self.apply_val(json_to_val(&input))
    }

    /// [`apply`](Self::apply) on an already parsed jq value.
    pub(crate) fn apply_val(&self, input: Val) -> Option<Value> {
        let values: Vec<Val> = self.run(input).collect::<Result<_, _>>().ok()?;
        match values.as_slice() {
            [] | [Val::Null] => None,
            [value] => Some(val_to_json(value)),
            values => Some(Value::Array(values.iter().map(val_to_json).collect())),
        }
    }

    /// Every value the filter outputs for `input`, skipping errors.
    pub(crate) fn outputs(&self, input: Val) -> Vec<Value> {
        self.run(input)
            .filter_map(Result::ok)
            .map(|value| val_to_json(&value))
            .collect()
    }

    /// Runs the filter on `input`, yielding each output value or error.
    fn run(&self, input: Val) -> ValXs<'_, Val> {
        let ctx = Ctx::<JqData>::new(&self.filter.lut, Vars::new([]));
        self.filter.id.run((ctx, input))
    }
}

impl Debug for JqExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("JqExpression")
            .field(&self.expression)
            .finish()
    }
}

/// Functions hitbox adds to the jq standard library.
///
/// Generic over the data type, like jaq's own function tables, so the
/// function pointers type-check for any data whose values are [`Val`].
fn hitbox_funs<D: for<'a> DataT<V<'a> = Val>>() -> impl Iterator<Item = Fun<D>> {
    // hash: hex SHA-256 of the input's string form
    let hash: RunPtr<D> = |cv| bome(Ok(Val::utf8_str(apply_hash(&hash_input(&cv.1)))));
    std::iter::once(native::run::<D>(("hash", v(0), hash)))
}

/// The string form `hash` digests.
///
/// Matches jaq-json 1, which hitbox used before: strings as is, numbers read
/// from JSON in `serde_json`'s rendering, computed floats via `Display`, and
/// arrays and objects as compact JSON.
fn hash_input(val: &Val) -> String {
    match val {
        Val::TStr(s) | Val::BStr(s) => String::from_utf8_lossy(s).into_owned(),
        Val::Num(Num::Int(n)) => n.to_string(),
        Val::Num(Num::Float(f)) => f.to_string(),
        Val::Bool(b) => b.to_string(),
        Val::Null => "null".to_owned(),
        Val::Num(Num::Dec(_) | Num::BigInt(_)) | Val::Arr(_) | Val::Obj(_) => {
            val_to_json(val).to_string()
        }
    }
}

/// A subject's body parsed for jq, kept in its [`Extensions`].
///
/// `None` records that the body is not JSON, so that is not re-checked either.
#[derive(Clone)]
pub(crate) struct ParsedBody(Option<Val>);

impl ParsedBody {
    /// Returns `body` parsed as JSON, parsing it only on the first call for a subject.
    ///
    /// Every jq predicate and extractor evaluating the same subject shares one
    /// parse. Callers pass the fully buffered body, which does not change
    /// during evaluation.
    pub(crate) fn get_or_parse(extensions: &mut Extensions, body: &[u8]) -> Option<Val> {
        if let Some(ParsedBody(val)) = extensions.get::<Self>() {
            return val.clone();
        }
        let val = read::parse_single(body).ok();
        extensions.insert(Self(val.clone()));
        val
    }

    /// Drops the parsed body once a subject leaves cache evaluation, so it is
    /// not carried upstream or back to the client.
    pub(crate) fn remove(extensions: &mut Extensions) {
        extensions.remove::<Self>();
    }
}

/// Converts a jq value to JSON the way jaq-json 1 did, so cache keys built from
/// jq output keep their format.
///
/// jaq-json 1 stored numbers that do not fit an `isize` as `serde_json`'s
/// rendering and parsed that back on output; parsing a decimal or big-integer
/// literal with `serde_json` gives the same number.
fn val_to_json(val: &Val) -> Value {
    match val {
        Val::Null => Value::Null,
        Val::Bool(b) => Value::Bool(*b),
        Val::Num(num) => num_to_json(num),
        Val::TStr(s) | Val::BStr(s) => Value::String(String::from_utf8_lossy(s).into_owned()),
        Val::Arr(items) => Value::Array(items.iter().map(val_to_json).collect()),
        Val::Obj(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key_to_string(key), val_to_json(value)))
                .collect(),
        ),
    }
}

fn num_to_json(num: &Num) -> Value {
    let number = match num {
        Num::Int(i) => Some(Number::from(*i)),
        Num::Float(f) => Number::from_f64(*f),
        Num::BigInt(i) => Number::from_str(&i.to_string()).ok(),
        Num::Dec(literal) => Number::from_str(literal).ok(),
    };
    number.map_or(Value::Null, Value::Number)
}

fn key_to_string(key: &Val) -> String {
    match key {
        Val::TStr(s) | Val::BStr(s) => String::from_utf8_lossy(s).into_owned(),
        other => val_to_json(other).to_string(),
    }
}

/// Converts JSON to a jq value with jaq-json 1's number handling: integers that
/// fit an `isize` stay integers, other numbers keep `serde_json`'s rendering.
fn json_to_val(value: &Value) -> Val {
    match value {
        Value::Null => Val::Null,
        Value::Bool(b) => Val::Bool(*b),
        Value::Number(n) => Val::Num(
            n.as_i64()
                .and_then(|i| isize::try_from(i).ok())
                .map_or_else(|| Num::Dec(Rc::new(n.to_string())), Num::Int),
        ),
        Value::String(s) => Val::from(s.clone()),
        Value::Array(items) => Val::Arr(Rc::new(items.iter().map(json_to_val).collect())),
        Value::Object(map) => Val::Obj(Rc::new(
            map.iter()
                .map(|(key, value)| (Val::from(key.clone()), json_to_val(value)))
                .collect(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BufferedBody;
    use crate::extractors::NeutralExtractor;
    use crate::extractors::body::{BodyExtraction, BodyExtractor};
    use crate::predicates::NeutralRequestPredicate;
    use crate::predicates::body::{BodyPredicate, JqOperation, Operation};
    use crate::{CacheableHttpRequest, CacheableHttpResponse, CacheableSubject};
    use bytes::Bytes;
    use hitbox::Extractor;
    use hitbox::predicate::{Predicate, PredicateResult};
    use http_body_util::Empty;

    fn complete(body: &'static [u8]) -> BufferedBody<Empty<Bytes>> {
        BufferedBody::Complete {
            data: Some(Bytes::from_static(body)),
            trailers: None,
        }
    }

    fn request(body: &'static [u8]) -> CacheableHttpRequest<Empty<Bytes>> {
        CacheableHttpRequest::from_request(http::Request::new(complete(body)))
    }

    fn parse(json: &str) -> Val {
        read::parse_single(json.as_bytes()).unwrap()
    }

    #[test]
    fn parsed_body_is_parsed_once_per_subject() {
        let mut extensions = Extensions::new();
        let first = ParsedBody::get_or_parse(&mut extensions, br#"{"a": 1}"#);
        // Same subject, so the stored parse is returned and these bytes are ignored.
        let second = ParsedBody::get_or_parse(&mut extensions, br#"{"a": 2}"#);
        assert_eq!(first, Some(parse(r#"{"a": 1}"#)));
        assert_eq!(second, first);
    }

    #[test]
    fn parsed_body_remembers_invalid_json() {
        let mut extensions = Extensions::new();
        assert_eq!(ParsedBody::get_or_parse(&mut extensions, b"not json"), None);
        assert_eq!(ParsedBody::get_or_parse(&mut extensions, b"{}"), None);
    }

    #[tokio::test]
    async fn extractor_reuses_body_parsed_by_predicate() {
        let predicate = NeutralRequestPredicate::new().body(Operation::Jq {
            filter: JqExpression::compile(".user.id").unwrap(),
            operation: JqOperation::Exist,
        });
        let PredicateResult::Cacheable(subject) =
            predicate.check(request(br#"{"user": {"id": 42}}"#)).await
        else {
            panic!("expected the jq predicate to match");
        };

        // Swap the stored parse: the extractor must read it instead of the body.
        let (mut parts, body) = subject.into_parts();
        assert!(parts.extensions.get::<ParsedBody>().is_some());
        parts
            .extensions
            .insert(ParsedBody(Some(parse(r#"{"user": {"id": 7}}"#))));
        let subject = CacheableHttpRequest::from_parts(parts, body);

        let extractor = NeutralExtractor::new().body(BodyExtraction::Jq(
            JqExpression::compile(".user.id").unwrap(),
        ));
        let (_, key) = extractor.get(subject).await.into_cache_key();
        let parts: Vec<_> = key.parts().map(|p| (p.key(), p.value())).collect();
        assert_eq!(parts, [("body", Some("7"))]);
    }

    #[test]
    fn into_request_and_into_response_drop_parsed_body() {
        let (mut parts, body) = request(b"{}").into_parts();
        ParsedBody::get_or_parse(&mut parts.extensions, b"{}");
        let request = CacheableHttpRequest::from_parts(parts, body).into_request();
        assert!(request.extensions().get::<ParsedBody>().is_none());

        let response = CacheableHttpResponse::from_response(http::Response::new(complete(b"{}")));
        let (mut parts, body) = response.into_parts();
        ParsedBody::get_or_parse(&mut parts.extensions, b"{}");
        let response = CacheableHttpResponse::from_parts(parts, body).into_response();
        assert!(response.extensions().get::<ParsedBody>().is_none());
    }

    /// Keys built from jq output keep jaq-json 1's rendering, which went
    /// through `serde_json`: numbers render as `serde_json` parses them.
    #[test]
    fn numbers_render_like_serde_json() {
        let literals = [
            "0",
            "-1",
            "42",
            "1.5",
            "1.50",
            "1.0",
            "-0.0",
            "1e2",
            "1E-3",
            "0.1",
            "9223372036854775807",
            "9223372036854775808",
            "18446744073709551615",
            "18446744073709551616",
            "123456789012345678901234567890",
            "1e308",
        ];
        for literal in literals {
            let expected: Value = serde_json::from_str(literal).unwrap();
            let val = parse(literal);
            assert_eq!(val_to_json(&val), expected, "{literal}");
            assert_eq!(hash_input(&val), expected.to_string(), "{literal}");
        }
    }
}
