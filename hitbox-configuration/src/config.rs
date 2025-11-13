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
    ConfigError, Request, RequestPredicate, Response, ResponsePredicate,
    endpoint::{Endpoint, RequestExtractor},
    extractors::Extractor,
    types::MaybeUndefined,
};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct ConfigEndpoint {
    #[serde(default)]
    pub request: MaybeUndefined<Request>,
    #[serde(default)]
    pub response: MaybeUndefined<Response>,
    #[serde(default)]
    pub extractors: MaybeUndefined<Vec<Extractor>>,
    pub policy: PolicyConfig,
}

impl ConfigEndpoint {
    pub fn extractors<ReqBody>(&self) -> RequestExtractor<ReqBody>
    where
        ReqBody: hyper::body::Body + Send + Debug + 'static,
        ReqBody::Error: Debug + Send,
        ReqBody::Data: Send,
    {
        match &self.extractors {
            MaybeUndefined::Null => Box::new(NeutralExtractor::<ReqBody>::new()),
            MaybeUndefined::Undefined => {
                Box::new(NeutralExtractor::<ReqBody>::new().method().path("*"))
            }
            MaybeUndefined::Value(extractors) => extractors.iter().cloned().rfold(
                Box::new(NeutralExtractor::<ReqBody>::new()),
                |inner, item| item.into_extractors(inner),
            ),
        }
    }

    pub fn into_endpoint<ReqBody, ResBody>(self) -> Result<Endpoint<ReqBody, ResBody>, ConfigError>
    where
        ReqBody: hyper::body::Body + Send + Debug + 'static,
        ReqBody::Error: Debug + Send,
        ReqBody::Data: Send,
        ResBody: hyper::body::Body + Send + 'static,
        ResBody::Error: Debug + Send,
        ResBody::Data: Send,
    {
        let extractors = Arc::new(self.extractors());
        let response_predicates = Arc::new(match self.response {
            MaybeUndefined::Value(response) => response.into_predicates()?,
            MaybeUndefined::Null => {
                Box::new(NeutralResponsePredicate::<ResBody>::new()) as ResponsePredicate<ResBody>
            }
            MaybeUndefined::Undefined => {
                Box::new(NeutralResponsePredicate::<ResBody>::new().status_code(StatusCode::OK))
                    as ResponsePredicate<ResBody>
            }
        });
        let request_predicates = Arc::new(match self.request {
            MaybeUndefined::Value(request) => request.into_predicates()?,
            MaybeUndefined::Null => {
                Box::new(NeutralRequestPredicate::<ReqBody>::new()) as RequestPredicate<ReqBody>
            }
            MaybeUndefined::Undefined => {
                Box::new(NeutralRequestPredicate::<ReqBody>::new().method(Method::GET))
                    as RequestPredicate<ReqBody>
            }
        });
        Ok(Endpoint {
            extractors,
            request_predicates,
            response_predicates,
            policy: self.policy,
        })
    }
}
