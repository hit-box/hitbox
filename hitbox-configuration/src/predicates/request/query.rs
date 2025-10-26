use hitbox_http::predicates::request::Query;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;

#[derive(Debug, Eq, PartialEq)]
pub struct QueryOperation {
    params: IndexMap<String, ParamOperation>,
}

#[derive(Debug, Eq, PartialEq)]
enum ParamOperation {
    Eq(String),
    In(Vec<String>),
    Exists,
}

impl QueryOperation {
    pub(crate) fn into_predicates<ReqBody: Send + 'static>(
        &self,
        inner: RequestPredicate<ReqBody>,
    ) -> RequestPredicate<ReqBody> {
        self.params.iter().rfold(inner, |inner, (key, param_op)| {
            match param_op {
                ParamOperation::Eq(value) => Box::new(Query::new(
                    inner,
                    hitbox_http::predicates::request::query::Operation::Eq(
                        key.clone(),
                        value.clone(),
                    ),
                )),
                ParamOperation::In(values) => Box::new(Query::new(
                    inner,
                    hitbox_http::predicates::request::query::Operation::In(
                        key.clone(),
                        values.clone(),
                    ),
                )),
                ParamOperation::Exists => Box::new(Query::new(
                    inner,
                    hitbox_http::predicates::request::query::Operation::Exist(key.clone()),
                )),
            }
        })
    }
}

// Custom deserialization to handle YAML tags
impl<'de> Deserialize<'de> for QueryOperation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        // NOTE: Deserializing to Value first loses location information.
        // Errors in parse_param_operation will report the location where
        // QueryOperation deserialization started, not where the actual error is.
        // We compensate by including parameter names in error messages.
        // See ERROR_LOCATION_LIMITATION.md for details.
        let value = serde_yaml::Value::deserialize(deserializer)?;

        let mapping = value
            .as_mapping()
            .ok_or_else(|| de::Error::custom("expected a mapping"))?;

        let mut params = IndexMap::new();

        for (key, val) in mapping {
            let key_str = key
                .as_str()
                .ok_or_else(|| de::Error::custom("expected string key"))?
                .to_string();

            let param_op = parse_param_operation(val)
                .map_err(|e| de::Error::custom(format!("parameter '{}': {}", key_str, e)))?;

            params.insert(key_str, param_op);
        }

        Ok(QueryOperation { params })
    }
}

// Custom serialization to preserve YAML tags
impl Serialize for QueryOperation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.params.len()))?;

        for (key, op) in &self.params {
            let value = param_operation_to_yaml(op);
            map.serialize_entry(key, &value)?;
        }

        map.end()
    }
}

fn parse_param_operation(value: &serde_yaml::Value) -> Result<ParamOperation, String> {
    match value {
        serde_yaml::Value::String(s) => Ok(ParamOperation::Eq(s.clone())),
        serde_yaml::Value::Number(n) => Ok(ParamOperation::Eq(n.to_string())),
        serde_yaml::Value::Bool(b) => Ok(ParamOperation::Eq(b.to_string())),
        serde_yaml::Value::Sequence(seq) => {
            let strings: Vec<String> = seq
                .iter()
                .filter_map(|v| {
                    v.as_str()
                        .map(|s| s.to_string())
                        .or_else(|| v.as_i64().map(|n| n.to_string()))
                        .or_else(|| v.as_bool().map(|b| b.to_string()))
                })
                .collect();
            Ok(ParamOperation::In(strings))
        }
        serde_yaml::Value::Tagged(tagged) => {
            let tag = tagged.tag.to_string();

            match tag.as_str() {
                "!exists" => Ok(ParamOperation::Exists),
                "!eq" => {
                    // Explicit Eq tag
                    let value_str = if let Some(s) = tagged.value.as_str() {
                        s.to_string()
                    } else if let Some(n) = tagged.value.as_i64() {
                        n.to_string()
                    } else if let Some(b) = tagged.value.as_bool() {
                        b.to_string()
                    } else {
                        return Err(format!("!eq expects a scalar value (string, number, or bool)"));
                    };
                    Ok(ParamOperation::Eq(value_str))
                }
                "!in" => {
                    let values = tagged
                        .value
                        .as_sequence()
                        .ok_or_else(|| format!("!in expects array"))?;
                    let strings: Vec<String> = values
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    Ok(ParamOperation::In(strings))
                }
                _ => Err(format!("Unknown tag: {}", tag)),
            }
        }
        _ => Err(format!("Unexpected value type: {:?}", value)),
    }
}

fn param_operation_to_yaml(op: &ParamOperation) -> serde_yaml::Value {
    match op {
        ParamOperation::Eq(s) => serde_yaml::Value::String(s.clone()),
        ParamOperation::In(values) => serde_yaml::Value::Sequence(
            values
                .iter()
                .map(|s| serde_yaml::Value::String(s.clone()))
                .collect(),
        ),
        ParamOperation::Exists => serde_yaml::Value::Tagged(Box::new(
            serde_yaml::value::TaggedValue {
                tag: serde_yaml::value::Tag::new("!exists"),
                value: serde_yaml::Value::Null,
            },
        )),
    }
}
