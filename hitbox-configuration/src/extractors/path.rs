use hitbox_http::extractors;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::RequestExtractor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, JsonSchema)]
pub struct Path(String);

impl Path {
    pub fn new(resource: impl Into<String>) -> Self {
        Self(resource.into())
    }

    pub fn into_extractors<ReqBody: Send + 'static>(
        self,
        inner: RequestExtractor<ReqBody>,
    ) -> RequestExtractor<ReqBody> {
        Box::new(extractors::Path::new(inner, self.0))
    }
}
