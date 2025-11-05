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
    pub fn into_predicates<ReqBody>(
        &self,
        inner: CorePredicate<ReqBody>,
    ) -> CorePredicate<ReqBody>
    where
        ReqBody: HttpBody + FromBytes + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
        match self {
            HeaderOperation::Eq(params) => params.iter().rfold(inner, |inner, (key, value)| {
                Box::new(Header::new(
                    inner,
                    hitbox_http::predicates::response::header::Operation::Eq(
                        key.parse().unwrap(),
                        value.parse().unwrap(),
                    ),
                ))
            }),
            HeaderOperation::Exist(param) => Box::new(Header::new(
                inner,
                hitbox_http::predicates::response::header::Operation::Exist(
                    param.parse().unwrap(),
                ),
            )),
            HeaderOperation::In(params) => params.iter().rfold(inner, |inner, (key, values)| {
                Box::new(Header::new(
                    inner,
                    hitbox_http::predicates::response::header::Operation::In(
                        key.parse().unwrap(),
                        values.iter().map(|v| v.parse().unwrap()).collect(),
                    ),
                ))
            }),
            HeaderOperation::Contains { contains } => {
                contains.iter().rfold(inner, |inner, (key, value)| {
                    Box::new(Header::new(
                        inner,
                        hitbox_http::predicates::response::header::Operation::Contains(
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
                    hitbox_http::predicates::response::header::Operation::Regex(
                        key.parse().unwrap(),
                        compiled_regex,
                    ),
                ))
            }),
        }
    }
}
