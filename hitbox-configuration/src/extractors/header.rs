use hitbox_http::extractors;
use serde::{Deserialize, Serialize};

use crate::RequestExtractor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Header(String);

impl Header {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn into_extractors<ReqBody: Send + 'static>(
        self,
        inner: RequestExtractor<ReqBody>,
    ) -> RequestExtractor<ReqBody> {
        Box::new(extractors::header::HeaderExtractor::header(inner, self.0))
    }
}
