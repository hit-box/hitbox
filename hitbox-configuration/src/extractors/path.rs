use hitbox_http::extractors;
use serde::{Deserialize, Serialize};

use crate::extractors::BoxExtractor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Path(String);

impl Path {
    pub fn new(resource: impl Into<String>) -> Self {
        Self(resource.into())
    }

    pub fn into_extractors<ReqBody: Send + 'static>(
        self,
        inner: BoxExtractor<ReqBody>,
    ) -> BoxExtractor<ReqBody> {
        Box::new(extractors::Path::new(inner, self.0))
    }
}
