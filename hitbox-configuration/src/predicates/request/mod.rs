use hitbox_http::FromBytes;
use hitbox_http::predicates::NeutralRequestPredicate;
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;

mod body;
mod expression;
mod header;
mod method;
mod operation;
mod predicate;
mod query;

pub use body::BodyPredicate;
pub use expression::Expression;
pub use header::HeaderOperation;
pub use method::MethodOperation;
pub use operation::Operation;
pub use predicate::Predicate;
pub use query::QueryOperation;

#[derive(Debug, Eq, PartialEq)]
pub enum Request {
    Flat(Vec<Predicate>),
    Tree(Expression),
}

// Custom serialization for Request to use untagged format
impl Serialize for Request {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Request::Flat(predicates) => predicates.serialize(serializer),
            Request::Tree(expression) => expression.serialize(serializer),
        }
    }
}

// Custom deserialization for Request to handle YAML tags in Predicates
impl<'de> Deserialize<'de> for Request {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        let value = serde_yaml::Value::deserialize(deserializer)?;

        // Try to deserialize as array (Flat)
        if value.is_sequence() {
            let predicates: Vec<Predicate> = serde_yaml::from_value(value)
                .map_err(|e| de::Error::custom(format!("Flat: {}", e)))?;
            return Ok(Request::Flat(predicates));
        }

        // Otherwise, try as Tree (Expression)
        let expression: Expression = serde_yaml::from_value(value)
            .map_err(|e| de::Error::custom(format!("Tree: {}", e)))?;
        Ok(Request::Tree(expression))
    }
}

impl Default for Request {
    fn default() -> Self {
        Self::Flat(Vec::default())
    }
}

impl Request {
    pub fn into_predicates<Req>(self) -> RequestPredicate<Req>
    where
        Req: HttpBody + FromBytes + Send + 'static,
        Req::Error: std::fmt::Debug,
        Req::Data: Send,
    {
        let neutral_predicate = Box::new(NeutralRequestPredicate::<Req>::new());
        match self {
            Request::Flat(predicates) => predicates
                .iter()
                .rfold(neutral_predicate, |inner, predicate| {
                    predicate.into_predicates(inner)
                }),
            Request::Tree(expression) => expression.into_predicates(neutral_predicate),
        }
    }

    pub fn predicates<Req>(&self) -> RequestPredicate<Req>
    where
        Req: HttpBody + FromBytes + Send + 'static,
        Req::Error: std::fmt::Debug,
        Req::Data: Send,
    {
        let neutral_predicate = Box::new(NeutralRequestPredicate::<Req>::new());
        match self {
            Request::Flat(predicates) => predicates
                .iter()
                .rfold(neutral_predicate, |inner, predicate| {
                    predicate.into_predicates(inner)
                }),
            Request::Tree(expression) => expression.into_predicates(neutral_predicate),
        }
    }
}
