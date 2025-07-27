use hitbox_http::extractors;
use serde::{Deserialize, Serialize};

use crate::extractors::BoxExtractor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Query(String);

impl Query {
    pub fn new(param: String) -> Self {
        Self(param)
    }

    pub fn into_extractors<ReqBody: Send + 'static>(
        self,
        inner: BoxExtractor<ReqBody>,
    ) -> BoxExtractor<ReqBody> {
        Box::new(extractors::query::Query::new(inner, self.0))
    }
}
