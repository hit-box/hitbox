use hitbox_http::extractors;
use serde::{Deserialize, Serialize};

use crate::extractors::BoxExtractor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Method {}

impl Method {
    pub fn new() -> Self {
        Self {}
    }

    pub fn into_extractors<ReqBody: Send + 'static>(
        self,
        inner: BoxExtractor<ReqBody>,
    ) -> BoxExtractor<ReqBody> {
        Box::new(extractors::Method::new(inner))
    }
}

impl Default for Method {
    fn default() -> Self {
        Self::new()
    }
}
