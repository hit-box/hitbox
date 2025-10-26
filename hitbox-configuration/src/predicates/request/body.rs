use hitbox_http::FromBytes;
use hitbox_http::predicates::request::BodyPredicate as _;
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum BodyPredicate {
    Jq(String),
    ProtoBuf {
        proto: String,
        message: String,
        expression: String,
    },
}

impl BodyPredicate {
    pub(crate) fn into_predicates<ReqBody>(
        &self,
        inner: RequestPredicate<ReqBody>,
    ) -> RequestPredicate<ReqBody>
    where
        ReqBody: HttpBody + FromBytes + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
        match self {
            BodyPredicate::Jq(expression) => Box::new(inner.body(
                hitbox_http::predicates::request::body::ParsingType::Jq,
                expression.clone(),
                // For Jq expressions, we expect the expression to evaluate to a boolean
                // (e.g., '.field == "value"' returns true/false)
                // We cache when the expression evaluates to true
                hitbox_http::predicates::request::body::Operation::Eq(serde_json::Value::Bool(
                    true,
                )),
            )),
            BodyPredicate::ProtoBuf {
                proto,
                message,
                expression: _,
            } => {
                // TODO: Load the MessageDescriptor from the proto file path
                // For now, this will panic if used - needs proper proto file loading
                todo!(
                    "ProtoBuf support requires loading .proto files from path: {} message: {}",
                    proto,
                    message
                )
            }
        }
    }
}
