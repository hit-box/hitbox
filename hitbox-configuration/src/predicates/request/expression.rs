use hitbox_http::FromBytes;
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;
use super::{Operation, Predicate};

// Use untagged enum - serde tries Operation first, then Predicate
#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Expression {
    Operation(Operation),
    Predicate(Predicate),
}

impl Expression {
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
            Self::Predicate(predicate) => predicate.into_predicates(inner),
            Self::Operation(operation) => operation.into_predicates(inner),
        }
    }
}
