use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::RequestExtractor;
use crate::error::ConfigError;
use crate::extractors::{
    body::BodyOperation, header::HeaderOperation, method::Method, path::Path,
    query::QueryOperation, version::Version,
};

pub mod body;
pub mod header;
pub mod method;
pub mod path;
pub mod query;
pub mod tag;
pub mod transform;
pub mod version;

pub use tag::{RequestTagExtractor, ResponseTagExtractor, TagsConfig};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum Extractor {
    Path(Path),
    Method(Method),
    Query(QueryOperation),
    Body(BodyOperation),
    Header(HeaderOperation),
    Version(Version),
}

impl Extractor {
    pub fn into_extractors<ReqBody>(
        self,
        inner: RequestExtractor<ReqBody>,
    ) -> Result<RequestExtractor<ReqBody>, ConfigError>
    where
        ReqBody: HttpBody + Send + 'static,
        ReqBody::Error: std::fmt::Debug + Send,
        ReqBody::Data: Send,
    {
        match self {
            Extractor::Method(method) => Ok(method.into_extractors(inner)),
            Extractor::Path(path) => Ok(path.into_extractors(inner)),
            Extractor::Query(query) => query.into_extractors(inner),
            Extractor::Body(body) => body.into_extractors(inner),
            Extractor::Header(header) => header.into_extractors(inner),
            Extractor::Version(version) => Ok(version.into_extractors(inner)),
        }
    }
}
