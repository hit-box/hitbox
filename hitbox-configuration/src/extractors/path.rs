use hitbox_http::extractors;
use serde::{Deserialize, Serialize};

use crate::RequestExtractor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Path(String);

impl Path {
    pub fn new(resource: impl Into<String>) -> Self {
        Self(resource.into())
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
        Box::new(extractors::Path::new(inner, self.0))
    }
}
