use hitbox_http::extractors::VersionConfig;
use hitbox_http::extractors::version::VersionExtractor;
use serde::{Deserialize, Serialize};

use crate::RequestExtractor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Version {}

impl Version {
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
        Box::new(inner.version(VersionConfig::new()))
    }
}

impl Default for Version {
    fn default() -> Self {
        Self::new()
    }
}
