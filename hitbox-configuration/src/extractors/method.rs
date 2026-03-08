use hitbox_http::extractors::MethodConfig;
use hitbox_http::extractors::method::MethodExtractor;
use serde::{Deserialize, Serialize};

use crate::RequestExtractor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Method {}

impl Method {
    pub fn new() -> Self {
        Self {}
    }

    pub fn into_extractors<ReqBody>(
        self,
        inner: RequestExtractor<ReqBody>,
    ) -> RequestExtractor<ReqBody>
    where
        ReqBody: hyper::body::Body + Send + 'static,
        ReqBody::Error: Send,
        ReqBody::Data: Send,
    {
        Box::new(inner.method(MethodConfig::new()))
    }
}

impl Default for Method {
    fn default() -> Self {
        Self::new()
    }
}
