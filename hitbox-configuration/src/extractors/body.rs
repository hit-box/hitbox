use hitbox_http::extractors;
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::RequestExtractor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
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
        ReqBody: HttpBody + Send + 'static,
        ReqBody::Error: std::fmt::Debug + Send,
        ReqBody::Data: Send,
    {
        Box::new(extractors::body::BodyExtractor::body(inner, self.0))
    }
}
