use hitbox::policy::PolicyConfig;
use hitbox_http::{CacheableHttpRequest, extractors::NeutralExtractor};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub use predicates::{request::Request, response::Response};

use crate::extractors::Extractor;

pub mod extractors;
pub mod predicates;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct Endpoint {
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub request: Request,
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub response: Response,
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub extractors: Vec<Extractor>,
    pub policy: PolicyConfig,
}

impl Endpoint {
    pub fn extractors<ReqBody>(
        &self,
    ) -> Box<dyn hitbox_core::Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync>
    where
        ReqBody: Send + Debug + 'static,
    {
        self.extractors.iter().cloned().rfold(
            Box::new(NeutralExtractor::<ReqBody>::new()),
            |inner, item| item.into_extractors(inner),
        )
    }
}

// impl<ReqBody, ResBody> CacheConfig<ReqBody, ResBody> for Endpoint
// where
//     ReqBody: Send + Debug + 'static,
//     ResBody: Send + 'static,
// {
//     type RequestBody = CacheableHttpRequest<ReqBody>;
//     type ResponseBody = CacheableHttpResponse<ResBody>;
//
//     fn request_predicates(&self) -> RequestPredicate<Self::RequestBody> {
//         self.request.into_predicates()
//     }
//
//     fn response_predicates(&self) -> ResponsePredicate<Self::ResponseBody> {
//         self.response.into_predicates()
//     }
//
//     fn extractors(&self) -> RequestExtractor<Self::RequestBody> {
//         self.extractors()
//     }
//
//     fn policy(&self) -> PolicyConfig {
//         PolicyConfig::default()
//     }
// }
