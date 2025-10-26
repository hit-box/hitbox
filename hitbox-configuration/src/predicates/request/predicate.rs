use hitbox_http::FromBytes;
use hitbox_http::predicates::request::Path;
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;
use super::{BodyPredicate, HeaderOperation, MethodOperation, QueryOperation};

#[derive(Debug, Eq, PartialEq)]
pub enum Predicate {
    Method(MethodOperation),
    Path(String),
    Query(QueryOperation),
    Header(HeaderOperation),
    Body(BodyPredicate),
}

// Custom serialization for Predicate to use map-key format (Method:, Path:, etc.)
impl Serialize for Predicate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(1))?;

        match self {
            Predicate::Method(method_op) => {
                map.serialize_entry("Method", method_op)?;
            }
            Predicate::Path(path) => {
                map.serialize_entry("Path", path)?;
            }
            Predicate::Query(query_op) => {
                map.serialize_entry("Query", query_op)?;
            }
            Predicate::Header(header_op) => {
                map.serialize_entry("Header", header_op)?;
            }
            Predicate::Body(body_pred) => {
                map.serialize_entry("Body", body_pred)?;
            }
        }

        map.end()
    }
}

// Custom deserialization for Predicate to properly handle QueryOperation's custom deserializer
impl<'de> Deserialize<'de> for Predicate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        let value = serde_yaml::Value::deserialize(deserializer)?;

        let mapping = value
            .as_mapping()
            .ok_or_else(|| de::Error::custom("expected a mapping for Predicate"))?;

        if mapping.len() != 1 {
            return Err(de::Error::custom(
                "Predicate must have exactly one variant",
            ));
        }

        let (key, val) = mapping.iter().next().unwrap();
        let variant = key
            .as_str()
            .ok_or_else(|| de::Error::custom("expected string key for Predicate variant"))?;

        match variant {
            "Method" => {
                let method_op: MethodOperation = serde_yaml::from_value(val.clone())
                    .map_err(|e| de::Error::custom(format!("Method: {}", e)))?;
                Ok(Predicate::Method(method_op))
            }
            "Path" => {
                let path = val
                    .as_str()
                    .ok_or_else(|| de::Error::custom("Path expects a string"))?
                    .to_string();
                Ok(Predicate::Path(path))
            }
            "Query" => {
                let query_op: QueryOperation = serde_yaml::from_value(val.clone())
                    .map_err(|e| de::Error::custom(format!("Query: {}", e)))?;
                Ok(Predicate::Query(query_op))
            }
            "Header" => {
                let header_op: HeaderOperation = serde_yaml::from_value(val.clone())
                    .map_err(|e| de::Error::custom(format!("Header: {}", e)))?;
                Ok(Predicate::Header(header_op))
            }
            "Body" => {
                let body_pred: BodyPredicate = serde_yaml::from_value(val.clone())
                    .map_err(|e| de::Error::custom(format!("Body: {}", e)))?;
                Ok(Predicate::Body(body_pred))
            }
            _ => Err(de::Error::custom(format!("Unknown Predicate variant: {}", variant))),
        }
    }
}

impl Predicate {
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
            Predicate::Method(method_operation) => method_operation.into_predicates(inner),
            Predicate::Path(path) => Box::new(Path::new(inner, path.as_str().into())),
            Predicate::Query(query_operation) => query_operation.into_predicates(inner),
            Predicate::Header(header_operation) => header_operation.into_predicates(inner),
            Predicate::Body(body_predicate) => body_predicate.into_predicates(inner),
        }
    }
}
