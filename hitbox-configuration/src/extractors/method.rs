use hitbox_http::extractors;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::RequestExtractor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, JsonSchema)]
pub struct Method {}

impl Method {
    pub fn new() -> Self {
        Self {}
    }

    pub fn into_extractors<ReqBody: Send + 'static>(
        self,
        inner: RequestExtractor<ReqBody>,
    ) -> RequestExtractor<ReqBody> {
        Box::new(extractors::Method::new(inner))
    }
}

impl Default for Method {
    fn default() -> Self {
        Self::new()
    }
}
