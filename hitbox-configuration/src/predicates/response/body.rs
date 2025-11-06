use hitbox_core::Predicate;
use hitbox_http::predicates::response::BodyPredicate;
use hitbox_http::predicates::response::body::{Operation as BodyOperation, ParsingType};
use hitbox_http::{CacheableHttpResponse, FromBytes};
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

type CorePredicate<ReqBody> =
    Box<dyn Predicate<Subject = CacheableHttpResponse<ReqBody>> + Send + Sync>;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Operation {
    Jq(String),
    ProtoBuf {
        proto: String,
        message: String,
        expression: String,
    },
}

impl Operation {
    pub fn into_predicates<ReqBody>(&self, inner: CorePredicate<ReqBody>) -> CorePredicate<ReqBody>
    where
        ReqBody: HttpBody + FromBytes + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
        match self {
            Operation::Jq(expression) => Box::new(inner.body(
                ParsingType::Jq,
                expression.clone(),
                // For Jq expressions, we expect the expression to evaluate to a boolean
                // (e.g., '.field == "value"' returns true/false)
                // We cache when the expression evaluates to true
                BodyOperation::Eq(JsonValue::Bool(true)),
            )),
            Operation::ProtoBuf {
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
