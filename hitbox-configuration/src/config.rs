use std::{fmt::Debug, sync::Arc};

use hitbox::policy::PolicyConfig;
use hitbox_http::{
    extractors::{NeutralExtractor, method::MethodExtractor, path::PathExtractor},
    predicates::{
        NeutralRequestPredicate, NeutralResponsePredicate, request::MethodPredicate,
        response::StatusCodePredicate,
    },
};
use http::{Method, StatusCode};
use serde::{Deserialize, Serialize};

use crate::{
    Request, RequestPredicate, Response, ResponsePredicate,
    endpoint::{Endpoint, RequestExtractor},
    extractors::Extractor,
    types::MaybeUndefined,
};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct ConfigEndpoint {
    #[serde(default)]
    pub request: MaybeUndefined<Request>,
    #[serde(default, with = "serde_yaml::with::singleton_map_recursive")]
    pub response: MaybeUndefined<Response>,
    #[serde(default, with = "serde_yaml::with::singleton_map_recursive")]
    pub extractors: MaybeUndefined<Vec<Extractor>>,
    pub policy: PolicyConfig,
}

impl ConfigEndpoint {
    pub fn extractors<ReqBody>(&self) -> RequestExtractor<ReqBody>
    where
        ReqBody: Send + Debug + 'static,
    {
        match &self.extractors {
            MaybeUndefined::Null => Box::new(NeutralExtractor::new()),
            MaybeUndefined::Undefined => Box::new(NeutralExtractor::new().method().path("*")),
            MaybeUndefined::Value(extractors) => extractors.iter().cloned().rfold(
                Box::new(NeutralExtractor::<ReqBody>::new()),
                |inner, item| item.into_extractors(inner),
            ),
        }
    }

    pub fn into_endpoint<ReqBody, ResBody>(self) -> Endpoint<ReqBody, ResBody>
    where
        ReqBody: hyper::body::Body + hitbox_http::FromBytes + Send + Debug + 'static,
        ReqBody::Error: Debug,
        ReqBody::Data: Send,
        ResBody: hyper::body::Body + hitbox_http::FromBytes + Send + 'static,
        ResBody::Error: Debug,
        ResBody::Data: Send,
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
        let request_predicates = Arc::new(match self.request {
            MaybeUndefined::Value(request) => request.into_predicates(),
            MaybeUndefined::Null => {
                Box::new(NeutralRequestPredicate::new()) as RequestPredicate<ReqBody>
            }
            MaybeUndefined::Undefined => {
                Box::new(NeutralRequestPredicate::new().method(Method::GET))
                    as RequestPredicate<ReqBody>
            }
        });
        Endpoint {
            extractors,
            request_predicates,
            response_predicates,
            policy: self.policy,
        }
    }
}
