use hitbox_http::FromBytes;
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;
use super::{Operation, Predicate};

#[derive(Debug, Eq, PartialEq)]
pub enum Expression {
    Predicate(Predicate),
    Operation(Operation),
}

// Custom serialization for Expression to use untagged format
impl Serialize for Expression {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Expression::Predicate(predicate) => predicate.serialize(serializer),
            Expression::Operation(operation) => operation.serialize(serializer),
        }
    }
}

// Custom deserialization for Expression to handle YAML tags in nested Predicates
impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        let value = serde_yaml::Value::deserialize(deserializer)?;

        // Try to deserialize as Operation first (And/Or/Not)
        if let Some(mapping) = value.as_mapping() {
            if mapping.len() == 1 {
                let (key, _) = mapping.iter().next().unwrap();
                if let Some(key_str) = key.as_str() {
                    if matches!(key_str, "And" | "Or" | "Not") {
                        let operation: Operation = serde_yaml::from_value(value)
                            .map_err(|e| de::Error::custom(format!("Operation: {}", e)))?;
                        return Ok(Expression::Operation(operation));
                    }
                }
            }
        }

        // Otherwise, try as Predicate
        let predicate: Predicate = serde_yaml::from_value(value)
            .map_err(|e| de::Error::custom(format!("Predicate: {}", e)))?;
        Ok(Expression::Predicate(predicate))
    }
}

impl Expression {
    pub fn into_predicates<ReqBody>(
        &self,
        inner: RequestPredicate<ReqBody>,
    ) -> RequestPredicate<ReqBody>
    where
        ReqBody: HttpBody + FromBytes + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
        match self {
            Self::Predicate(predicate) => predicate.into_predicates(inner),
            Self::Operation(operation) => operation.into_predicates(inner),
        }
    }
}
