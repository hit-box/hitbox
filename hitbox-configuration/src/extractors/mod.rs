use hitbox_http::CacheableHttpRequest;
use serde::{Deserialize, Serialize};

use crate::extractors::{method::Method, path::Path};

pub mod method;
pub mod path;

pub type BoxExtractor<ReqBody> =
    Box<dyn hitbox_core::Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Extractor {
    Path(Path),
    Method(Method),
}

impl Extractor {
    pub fn into_extractors<ReqBody>(self, inner: BoxExtractor<ReqBody>) -> BoxExtractor<ReqBody>
    where
        ReqBody: Send + 'static,
    {
        match self {
            Extractor::Method(method) => method.into_extractors(inner),
            Extractor::Path(path) => path.into_extractors(inner),
        }
    }
}
