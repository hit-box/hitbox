use hitbox_http::predicates::request::Header;
use indexmap::IndexMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum HeaderOperation {
    // Explicit-only operations (must come first to match correctly)
    Contains { contains: IndexMap<String, String> },
    Regex { regex: IndexMap<String, String> },

    // Implicit + explicit operations
    Eq(IndexMap<String, String>),
    In(IndexMap<String, Vec<String>>),

    // Single string = Exist (must be last)
    Exist(String),
}

impl HeaderOperation {
    pub(crate) fn into_predicates<ReqBody: Send + 'static>(
        &self,
        inner: RequestPredicate<ReqBody>,
    ) -> RequestPredicate<ReqBody> {
        match self {
            HeaderOperation::Eq(params) => params.iter().rfold(inner, |inner, (key, value)| {
                Box::new(Header::new(
                    inner,
                    hitbox_http::predicates::request::header::Operation::Eq(
                        key.parse().unwrap(),
                        value.parse().unwrap(),
                    ),
                ))
            }),
            HeaderOperation::Exist(param) => Box::new(Header::new(
                inner,
                hitbox_http::predicates::request::header::Operation::Exist(param.parse().unwrap()),
            )),
            HeaderOperation::In(params) => params.iter().rfold(inner, |inner, (key, values)| {
                Box::new(Header::new(
                    inner,
                    hitbox_http::predicates::request::header::Operation::In(
                        key.parse().unwrap(),
                        values.iter().map(|v| v.parse().unwrap()).collect(),
                    ),
                ))
            }),
            HeaderOperation::Contains { contains } => {
                contains.iter().rfold(inner, |inner, (key, value)| {
                    Box::new(Header::new(
                        inner,
                        hitbox_http::predicates::request::header::Operation::Contains(
                            key.parse().unwrap(),
                            value.clone(),
                        ),
                    ))
                })
            }
            HeaderOperation::Regex { regex } => regex.iter().rfold(inner, |inner, (key, pattern)| {
                let compiled_regex = Regex::new(pattern).expect("Invalid regex pattern");
                Box::new(Header::new(
                    inner,
                    hitbox_http::predicates::request::header::Operation::Regex(
                        key.parse().unwrap(),
                        compiled_regex,
                    ),
                ))
            }),
        }
    }
}
