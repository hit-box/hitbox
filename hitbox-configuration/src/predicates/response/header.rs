use hitbox_http::predicates::response::Header;
use indexmap::IndexMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

use hitbox_core::Predicate;
use hitbox_http::{CacheableHttpResponse, FromBytes};
use hyper::body::Body as HttpBody;

type CorePredicate<ReqBody> =
    Box<dyn Predicate<Subject = CacheableHttpResponse<ReqBody>> + Send + Sync>;

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
    inner: CorePredicate<ReqBody>,
) -> CorePredicate<ReqBody>
where
    ReqBody: HttpBody + FromBytes + Send + 'static,
    ReqBody::Error: std::fmt::Debug,
    ReqBody::Data: Send,
{
    headers
        .into_iter()
        .rfold(inner, |inner, (header_name, header_value)| {
            let operation = match header_value {
                HeaderValue::Eq(value) => hitbox_http::predicates::response::header::Operation::Eq(
                    header_name.parse().unwrap(),
                    value.parse().unwrap(),
                ),
                HeaderValue::In(values) => {
                    hitbox_http::predicates::response::header::Operation::In(
                        header_name.parse().unwrap(),
                        values.into_iter().map(|v| v.parse().unwrap()).collect(),
                    )
                }
                HeaderValue::Operation(op) => match op {
                    HeaderValueOperation::Eq(value) => {
                        hitbox_http::predicates::response::header::Operation::Eq(
                            header_name.parse().unwrap(),
                            value.parse().unwrap(),
                        )
                    }
                    HeaderValueOperation::In(values) => {
                        hitbox_http::predicates::response::header::Operation::In(
                            header_name.parse().unwrap(),
                            values.into_iter().map(|v| v.parse().unwrap()).collect(),
                        )
                    }
                    HeaderValueOperation::Contains(substring) => {
                        hitbox_http::predicates::response::header::Operation::Contains(
                            header_name.parse().unwrap(),
                            substring,
                        )
                    }
                    HeaderValueOperation::Regex(pattern) => {
                        let compiled_regex = Regex::new(&pattern).expect("Invalid regex pattern");
                        hitbox_http::predicates::response::header::Operation::Regex(
                            header_name.parse().unwrap(),
                            compiled_regex,
                        )
                    }
                    HeaderValueOperation::Exist => {
                        hitbox_http::predicates::response::header::Operation::Exist(
                            header_name.parse().unwrap(),
                        )
                    }
                },
            };
            Box::new(Header::new(inner, operation))
        })
}
