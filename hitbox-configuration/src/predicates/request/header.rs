use hitbox_http::predicates::request::Header;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum HeaderOperation {
    Eq(IndexMap<String, String>),
    Exist(String),
    In(IndexMap<String, Vec<String>>),
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
        }
    }
}
