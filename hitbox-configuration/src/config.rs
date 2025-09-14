use std::{fmt::Debug, sync::Arc};

use hitbox::policy::PolicyConfig;
use hitbox_http::{
    extractors::NeutralExtractor,
    predicates::{NeutralResponsePredicate, response::StatusCodePredicate},
};
use http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    Request, Response, ResponsePredicate,
    endpoint::{Endpoint, RequestExtractor},
    extractors::Extractor,
    types::MaybeUndefined,
};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct ConfigEndpoint {
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub request: Request,
    #[serde(default, with = "serde_yaml::with::singleton_map_recursive")]
    pub response: MaybeUndefined<Response>,
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
        let extractors = Arc::new(self.extractors());
        let response_predicates = Arc::new(match self.response {
            MaybeUndefined::Value(response) => response.into_predicates(),
            MaybeUndefined::Null => {
                Box::new(NeutralResponsePredicate::new()) as ResponsePredicate<ResBody>
            }
            MaybeUndefined::Undefined => {
                Box::new(NeutralResponsePredicate::new().status_code(StatusCode::OK))
                    as ResponsePredicate<ResBody>
            }
        });
        Endpoint {
            extractors,
            request_predicates: Arc::new(self.request.into_predicates()),
            response_predicates,
            policy: self.policy,
        }
    }
}
