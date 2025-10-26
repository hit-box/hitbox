use hitbox_http::FromBytes;
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::predicates::conditions::{Not, Or};
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;
use super::Expression;

#[derive(Debug, Eq, PartialEq)]
pub enum Operation {
    And(Vec<Expression>),
    Or(Vec<Expression>),
    Not(Box<Expression>),
}

// Custom serialization for Operation to use map-key format (And:, Or:, Not:)
impl Serialize for Operation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(1))?;

        match self {
            Operation::And(expressions) => {
                map.serialize_entry("And", expressions)?;
            }
            Operation::Or(expressions) => {
                map.serialize_entry("Or", expressions)?;
            }
            Operation::Not(expression) => {
                map.serialize_entry("Not", expression)?;
            }
        }

        map.end()
    }
}

// Custom deserialization for Operation to handle YAML map-key syntax (And:, Or:, Not:)
impl<'de> Deserialize<'de> for Operation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        let value = serde_yaml::Value::deserialize(deserializer)?;

        let mapping = value
            .as_mapping()
            .ok_or_else(|| de::Error::custom("expected a mapping for Operation"))?;

        if mapping.len() != 1 {
            return Err(de::Error::custom(
                "Operation must have exactly one variant",
            ));
        }

        let (key, val) = mapping.iter().next().unwrap();
        let variant = key
            .as_str()
            .ok_or_else(|| de::Error::custom("expected string key for Operation variant"))?;

        match variant {
            "And" => {
                let expressions: Vec<Expression> = serde_yaml::from_value(val.clone())
                    .map_err(|e| de::Error::custom(format!("And: {}", e)))?;
                Ok(Operation::And(expressions))
            }
            "Or" => {
                let expressions: Vec<Expression> = serde_yaml::from_value(val.clone())
                    .map_err(|e| de::Error::custom(format!("Or: {}", e)))?;
                Ok(Operation::Or(expressions))
            }
            "Not" => {
                let expression: Expression = serde_yaml::from_value(val.clone())
                    .map_err(|e| de::Error::custom(format!("Not: {}", e)))?;
                Ok(Operation::Not(Box::new(expression)))
            }
            _ => Err(de::Error::custom(format!("Unknown Operation variant: {}", variant))),
        }
    }
}

impl Operation {
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
            Operation::Or(predicates) => {
                let mut iter = predicates.iter();
                match iter.next() {
                    None => inner,
                    Some(first) => {
                        let first_predicate =
                            first.into_predicates(Box::new(NeutralRequestPredicate::new()));
                        iter.fold(first_predicate, |acc, predicate| {
                            let predicate =
                                predicate
                                    .into_predicates(Box::new(
                                        NeutralRequestPredicate::<ReqBody>::new(),
                                    ));
                            Box::new(Or::new(
                                predicate,
                                acc,
                                Box::new(NeutralRequestPredicate::new()),
                            ))
                        })
                    }
                }
            }
            Operation::And(predicates) => predicates
                .iter()
                .rfold(inner, |inner, predicate| predicate.into_predicates(inner)),
            Operation::Not(expression) => {
                let predicate =
                    expression.into_predicates(Box::new(NeutralRequestPredicate::new()));
                Box::new(Not::new(predicate))
            }
        }
    }
}
