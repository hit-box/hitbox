use hitbox_http::FromBytes;
use hitbox_http::predicates::request::Path;
use hyper::body::Body as HttpBody;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{BodyPredicate, HeaderOperation, MethodOperation, QueryOperation, header};
use crate::{RequestPredicate, error::ConfigError};

// Use standard externally-tagged enum (serde default)
// YAML syntax: Method: {...}, Path: "...", Query: {...}, etc.
#[derive(Debug, Eq, PartialEq, Deserialize, Serialize, JsonSchema)]
pub enum Predicate {
    Method(MethodOperation),
    Path(String),
    Query(QueryOperation),
    Header(HeaderOperation),
    Body(BodyPredicate),
}

impl Predicate {
    pub fn into_predicates<ReqBody>(
        self,
        inner: RequestPredicate<ReqBody>,
    ) -> Result<RequestPredicate<ReqBody>, ConfigError>
    where
        ReqBody: HttpBody + FromBytes + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
        match self {
            Predicate::Method(method_operation) => method_operation.into_predicates(inner),
            Predicate::Path(path) => Ok(Box::new(Path::new(inner, path.into()))),
            Predicate::Query(query_operation) => query_operation.into_predicates(inner),
            Predicate::Header(header_operation) => header::into_predicates(header_operation, inner),
            Predicate::Body(body_predicate) => body_predicate.into_predicates(inner),
        }
    }
}
