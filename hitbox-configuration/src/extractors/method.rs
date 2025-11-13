use hitbox_http::extractors;
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
        Box::new(extractors::Method::new(inner))
    }
}

impl Default for Method {
    fn default() -> Self {
        Self::new()
    }
}
