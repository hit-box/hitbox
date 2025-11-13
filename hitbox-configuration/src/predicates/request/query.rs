use std::fmt::Display;

use hitbox_http::predicates::request::Query;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::{RequestPredicate, error::ConfigError};

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct QueryOperation {
    #[serde(flatten)]
    params: IndexMap<String, ParamOperation>,
}

// Untagged enum to accept any scalar type and convert to String
// HTTP query parameters are always strings, but YAML allows different scalar types
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
enum QueryParamValue {
    String(String),
    Integer(i64),
    Boolean(bool),
}

impl Display for QueryParamValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryParamValue::String(s) => f.write_str(s),
            QueryParamValue::Integer(i) => write!(f, "{}", i),
            QueryParamValue::Boolean(b) => write!(f, "{}", b),
        }
    }
}

// Untagged enum for query parameter operations
#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
enum ParamOperation {
    // Try explicit forms first (must be mappings)
    ExplicitEq { eq: QueryParamValue },
    ExplicitIn { r#in: Vec<QueryParamValue> },
    // exists can be any value (or empty) - we ignore the value
    ExplicitExists { exists: serde_json::Value },

    // Fallback to implicit shortcuts
    ImplicitIn(Vec<QueryParamValue>), // Array → In
    ImplicitEq(QueryParamValue),      // Scalar → Eq (string, number, bool, etc.)
                                      // Note: No ImplicitExists - must use explicit {exists:}
}

impl ParamOperation {
    // Convert to the internal operation representation
    fn to_operation(&self) -> Operation {
        match self {
            ParamOperation::ExplicitEq { eq } => Operation::Eq(eq.to_string()),
            ParamOperation::ImplicitEq(eq) => Operation::Eq(eq.to_string()),
            ParamOperation::ExplicitIn { r#in } => {
                Operation::In(r#in.iter().map(|v| v.to_string()).collect())
            }
            ParamOperation::ImplicitIn(r#in) => {
                Operation::In(r#in.iter().map(|v| v.to_string()).collect())
            }
            ParamOperation::ExplicitExists { .. } => Operation::Exists,
        }
    }
}

// Internal canonical representation
#[derive(Debug, Eq, PartialEq, Clone)]
enum Operation {
    Eq(String),
    In(Vec<String>),
    Exists,
}

impl QueryOperation {
    pub fn into_predicates<ReqBody>(
        self,
        inner: RequestPredicate<ReqBody>,
    ) -> Result<RequestPredicate<ReqBody>, ConfigError>
    where
        ReqBody: hyper::body::Body + Send + 'static,
        ReqBody::Error: std::fmt::Debug + Send,
        ReqBody::Data: Send,
    {
        Ok(self
            .params
            .into_iter()
            .rfold(inner, |inner, (key, param_op)| {
                let op = param_op.to_operation();
                match op {
                    Operation::Eq(value) => Box::new(Query::new(
                        inner,
                        hitbox_http::predicates::request::query::Operation::Eq(key, value),
                    )),
                    Operation::In(values) => Box::new(Query::new(
                        inner,
                        hitbox_http::predicates::request::query::Operation::In(key, values),
                    )),
                    Operation::Exists => Box::new(Query::new(
                        inner,
                        hitbox_http::predicates::request::query::Operation::Exist(key),
                    )),
                }
            }))
    }
}
