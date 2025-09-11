use std::{fmt::Debug, sync::Arc};

use hitbox::policy::PolicyConfig;
use hitbox_http::extractors::NeutralExtractor;
use serde::{Deserialize, Serialize};

use crate::{
    Request, Response,
    endpoint::{Endpoint, RequestExtractor},
    extractors::Extractor,
};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct ConfigEndpoint {
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub request: Request,
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub response: Response,
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub extractors: Vec<Extractor>,
    pub policy: PolicyConfig,
}

impl ConfigEndpoint {
    pub fn extractors<ReqBody>(&self) -> RequestExtractor<ReqBody>
    where
        ReqBody: Send + Debug + 'static,
    {
        self.extractors.iter().cloned().rfold(
            Box::new(NeutralExtractor::<ReqBody>::new()),
            |inner, item| item.into_extractors(inner),
        )
    }

    pub fn into_endpoint<ReqBody, ResBody>(self) -> Endpoint<ReqBody, ResBody>
    where
        ReqBody: Send + Debug + 'static,
        ResBody: Send + 'static,
    {
        Endpoint {
            extractors: Arc::new(self.extractors()),
            request_predicates: Arc::new(self.request.into_predicates()),
            response_predicates: Arc::new(self.response.into_predicates()),
            policy: self.policy,
        }
    }
}
