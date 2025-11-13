use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::RequestExtractor;
use crate::extractors::{body::Body, header::Header, method::Method, path::Path, query::Query};

pub mod body;
pub mod header;
pub mod method;
pub mod path;
pub mod query;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Extractor {
    Path(Path),
    Method(Method),
    Query(Query),
    Body(Body),
    Header(Header),
}

impl Extractor {
    pub fn into_extractors<ReqBody>(
        self,
        inner: RequestExtractor<ReqBody>,
    ) -> RequestExtractor<ReqBody>
    where
        ReqBody: HttpBody + Send + 'static,
        ReqBody::Error: std::fmt::Debug + Send,
        ReqBody::Data: Send,
    {
        match self {
            Extractor::Method(method) => method.into_extractors(inner),
            Extractor::Path(path) => path.into_extractors(inner),
            Extractor::Query(query) => query.into_extractors(inner),
            Extractor::Body(body) => body.into_extractors(inner),
            Extractor::Header(header) => header.into_extractors(inner),
        }
    }
}
