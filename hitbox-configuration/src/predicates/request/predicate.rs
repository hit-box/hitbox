use hitbox_http::FromBytes;
use hitbox_http::predicates::request::Path;
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;
use super::{BodyPredicate, HeaderOperation, MethodOperation, QueryOperation, header};

// Use standard externally-tagged enum (serde default)
// YAML syntax: Method: {...}, Path: "...", Query: {...}, etc.
#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum Predicate {
    Method(MethodOperation),
    Path(String),
    Query(QueryOperation),
    Header(HeaderOperation),
    Body(BodyPredicate),
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
            Predicate::Header(header_operation) => header::into_predicates(header_operation, inner),
            Predicate::Body(body_predicate) => body_predicate.into_predicates(inner),
        }
    }
}
