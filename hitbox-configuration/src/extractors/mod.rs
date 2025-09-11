use serde::{Deserialize, Serialize};

use crate::RequestExtractor;
use crate::extractors::{method::Method, path::Path, query::Query};

pub mod method;
pub mod path;
pub mod query;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Extractor {
    Path(Path),
    Method(Method),
    Query(Query),
}

impl Extractor {
    pub fn into_extractors<ReqBody>(
        self,
        inner: RequestExtractor<ReqBody>,
    ) -> RequestExtractor<ReqBody>
    where
        ReqBody: Send + 'static,
    {
        match self {
            Extractor::Method(method) => method.into_extractors(inner),
            Extractor::Path(path) => path.into_extractors(inner),
            Extractor::Query(query) => query.into_extractors(inner),
        }
    }
}
