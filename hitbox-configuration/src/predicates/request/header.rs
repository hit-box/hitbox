use hitbox_http::predicates::request::{Header, header::Operation};
use http::header::{HeaderName, HeaderValue as HttpHeaderValue};
use indexmap::IndexMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{RequestPredicate, error::ConfigError};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum HeaderValue {
    Eq(String),
    In(Vec<String>),
    Operation(HeaderValueOperation),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HeaderValueOperation {
    Eq(String),
    In(Vec<String>),
    Contains(String),
    Regex(String),
    #[serde(deserialize_with = "deserialize_exist")]
    Exist,
}

fn deserialize_exist<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::IgnoredAny;
    IgnoredAny::deserialize(deserializer)?;
    Ok(())
}

pub type HeaderOperation = IndexMap<String, HeaderValue>;

pub fn into_predicates<ReqBody>(
    headers: HeaderOperation,
    inner: RequestPredicate<ReqBody>,
) -> Result<RequestPredicate<ReqBody>, ConfigError>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    ReqBody::Data: Send,
{
    headers.into_iter().try_rfold(
        inner,
        |inner, (header_name, header_value)| -> Result<RequestPredicate<ReqBody>, ConfigError> {
            let name = parse_header_name(&header_name)?;

            let operation = match header_value {
                HeaderValue::Eq(value) => {
                    let val = parse_header_value(&value)?;
                    Operation::Eq(name, val)
                }
                HeaderValue::In(values) => {
                    let vals = parse_header_values(&values)?;
                    Operation::In(name, vals)
                }
                HeaderValue::Operation(op) => match op {
                    HeaderValueOperation::Eq(value) => {
                        let val = parse_header_value(&value)?;
                        Operation::Eq(name, val)
                    }
                    HeaderValueOperation::In(values) => {
                        let vals = parse_header_values(&values)?;
                        Operation::In(name, vals)
                    }
                    HeaderValueOperation::Contains(substring) => {
                        Operation::Contains(name, substring)
                    }
                    HeaderValueOperation::Regex(pattern) => {
                        let compiled_regex =
                            Regex::new(&pattern).map_err(|e| ConfigError::InvalidRegex {
                                pattern: pattern.clone(),
                                error: e,
                            })?;
                        Operation::Regex(name, compiled_regex)
                    }
                    HeaderValueOperation::Exist => Operation::Exist(name),
                },
            };
            Ok(Box::new(Header::new(inner, operation)))
        },
    )
}

fn parse_header_name(name: &str) -> Result<HeaderName, ConfigError> {
    name.parse()
        .map_err(|e| ConfigError::InvalidHeaderName(name.to_string(), e))
}

fn parse_header_value(value: &str) -> Result<HttpHeaderValue, ConfigError> {
    value
        .parse()
        .map_err(|e| ConfigError::InvalidHeaderValue(value.to_string(), e))
}

fn parse_header_values(values: &[String]) -> Result<Vec<HttpHeaderValue>, ConfigError> {
    values.iter().map(|v| parse_header_value(v)).collect()
}
