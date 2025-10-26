use std::num::NonZeroU16;

use hitbox_core::Predicate;
use hitbox_http::predicates::response::{StatusClass, StatusCode};
use hitbox_http::{CacheableHttpResponse, FromBytes};
use http::StatusCode as HttpStatusCode;
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

type CorePredicate<ReqBody> =
    Box<dyn Predicate<Subject = CacheableHttpResponse<ReqBody>> + Send + Sync>;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Operation {
    Eq(NonZeroU16),
    In(Vec<NonZeroU16>),
    Range([NonZeroU16; 2]),
    Class(StatusClass),
}

impl Operation {
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
            Operation::Eq(code) => {
                Box::new(StatusCode::new(inner, code.get().try_into().unwrap()))
            }
            Operation::In(codes) => {
                let status_codes: Vec<HttpStatusCode> = codes
                    .iter()
                    .map(|c| c.get().try_into().unwrap())
                    .collect();
                Box::new(StatusCode::new_in(inner, status_codes))
            }
            Operation::Range([start, end]) => {
                Box::new(StatusCode::new_range(
                    inner,
                    start.get().try_into().unwrap(),
                    end.get().try_into().unwrap(),
                ))
            }
            Operation::Class(class) => Box::new(StatusCode::new_class(inner, *class)),
        }
    }
}
