use hitbox_http::FromBytes;
use hitbox_http::extractors;
use hyper::body::Body as HttpBody;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::RequestExtractor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, JsonSchema)]
pub struct Body(String);

impl Body {
    pub fn new(expression: impl Into<String>) -> Self {
        Self(expression.into())
    }

    pub fn into_extractors<ReqBody>(
        self,
        inner: RequestExtractor<ReqBody>,
    ) -> RequestExtractor<ReqBody>
    where
        ReqBody: HttpBody + FromBytes + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
        Box::new(extractors::body::BodyExtractor::body(inner, self.0))
    }
}
