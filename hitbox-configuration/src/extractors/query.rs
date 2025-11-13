use hitbox_http::extractors;
use serde::{Deserialize, Serialize};

use crate::RequestExtractor;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Query(String);

impl Query {
    pub fn new(param: String) -> Self {
        Self(param)
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
        Box::new(extractors::query::Query::new(inner, self.0))
    }
}
