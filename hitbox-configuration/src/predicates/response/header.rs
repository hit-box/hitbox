use hitbox_http::predicates::response::Header;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use hitbox_core::Predicate;
use hitbox_http::{CacheableHttpResponse, FromBytes};
use hyper::body::Body as HttpBody;

type CorePredicate<ReqBody> =
    Box<dyn Predicate<Subject = CacheableHttpResponse<ReqBody>> + Send + Sync>;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum HeaderOperation {
    Eq(IndexMap<String, String>),
    Exist(String),
    In(IndexMap<String, Vec<String>>),
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
        }
    }
}
