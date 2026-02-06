use std::{fmt::Debug, sync::Arc};

use hitbox::{config::MultiConfig, policy::PolicyConfig};
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

/// Single route configuration for a cache layer.
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct ConfigRoute {
    #[serde(default)]
    pub request: MaybeUndefined<Request>,
    #[serde(default)]
    pub response: MaybeUndefined<Response>,
    #[serde(default)]
    pub extractors: MaybeUndefined<Vec<Extractor>>,
    #[serde(default)]
    pub policy: PolicyConfig,
}

/// Config endpoint that supports both single-config and multi-config (`routes`) modes.
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct ConfigEndpoint {
    #[serde(default)]
    pub request: MaybeUndefined<Request>,
    #[serde(default)]
    pub response: MaybeUndefined<Response>,
    #[serde(default)]
    pub extractors: MaybeUndefined<Vec<Extractor>>,
    #[serde(default)]
    pub policy: PolicyConfig,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<ConfigRoute>,
}

impl ConfigEndpoint {
    fn has_inline_configuration(&self) -> bool {
        !matches!(self.request, MaybeUndefined::Undefined)
            || !matches!(self.response, MaybeUndefined::Undefined)
            || !matches!(self.extractors, MaybeUndefined::Undefined)
            || self.policy != PolicyConfig::default()
    }

    fn into_inline_route(self) -> ConfigRoute {
        ConfigRoute {
            request: self.request,
            response: self.response,
            extractors: self.extractors,
            policy: self.policy,
        }
    }

    pub fn extractors<ReqBody>(&self) -> Result<RequestExtractor<ReqBody>, ConfigError>
    where
        ReqBody: hyper::body::Body + Send + Debug + 'static,
        ReqBody::Error: Debug + Send,
        ReqBody::Data: Send,
    {
        build_extractors(&self.extractors)
    }

    pub fn into_endpoint<ReqBody, ResBody>(self) -> Result<Endpoint<ReqBody, ResBody>, ConfigError>
    where
        ReqBody: hyper::body::Body + Send + Unpin + Debug + 'static,
        ReqBody::Error: Debug + Send,
        ReqBody::Data: Send,
        ResBody: hyper::body::Body + Send + Unpin + 'static,
        ResBody::Error: Debug + Send,
        ResBody::Data: Send,
    {
        let has_inline_configuration = self.has_inline_configuration();
        let route_count = self.routes.len();

        if route_count == 0 {
            return self.into_inline_route().into_endpoint();
        }

        if has_inline_configuration {
            return Err(ConfigError::MixedRouteAndInlineConfig);
        }

        if route_count > 1 {
            return Err(ConfigError::MultipleRouteConfigurations(route_count));
        }

        self.routes
            .into_iter()
            .next()
            .expect("route_count checked")
            .into_endpoint()
    }

    pub fn into_routed_endpoint<ReqBody, ResBody>(
        self,
    ) -> Result<MultiConfig<Endpoint<ReqBody, ResBody>>, ConfigError>
    where
        ReqBody: hyper::body::Body + Send + Unpin + Debug + 'static,
        ReqBody::Error: Debug + Send,
        ReqBody::Data: Send,
        ResBody: hyper::body::Body + Send + Unpin + 'static,
        ResBody::Error: Debug + Send,
        ResBody::Data: Send,
    {
        if self.routes.is_empty() {
            return Ok(MultiConfig::new(vec![
                self.into_inline_route().into_endpoint()?,
            ]));
        }

        if self.has_inline_configuration() {
            return Err(ConfigError::MixedRouteAndInlineConfig);
        }

        let endpoints = self
            .routes
            .into_iter()
            .map(ConfigRoute::into_endpoint)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(MultiConfig::new(endpoints))
    }
}

impl ConfigRoute {
    pub fn extractors<ReqBody>(&self) -> Result<RequestExtractor<ReqBody>, ConfigError>
    where
        ReqBody: hyper::body::Body + Send + Debug + 'static,
        ReqBody::Error: Debug + Send,
        ReqBody::Data: Send,
    {
        build_extractors(&self.extractors)
    }

    pub fn into_endpoint<ReqBody, ResBody>(self) -> Result<Endpoint<ReqBody, ResBody>, ConfigError>
    where
        ReqBody: hyper::body::Body + Send + Unpin + Debug + 'static,
        ReqBody::Error: Debug + Send,
        ReqBody::Data: Send,
        ResBody: hyper::body::Body + Send + Unpin + 'static,
        ResBody::Error: Debug + Send,
        ResBody::Data: Send,
    {
        build_endpoint(self.request, self.response, self.extractors, self.policy)
    }
}

fn build_extractors<ReqBody>(
    extractors: &MaybeUndefined<Vec<Extractor>>,
) -> Result<RequestExtractor<ReqBody>, ConfigError>
where
    ReqBody: hyper::body::Body + Send + Debug + 'static,
    ReqBody::Error: Debug + Send,
    ReqBody::Data: Send,
{
    match extractors {
        MaybeUndefined::Null => Ok(Box::new(NeutralExtractor::<ReqBody>::new())),
        MaybeUndefined::Undefined => Ok(Box::new(
            NeutralExtractor::<ReqBody>::new().method().path("{path}*"),
        )),
        MaybeUndefined::Value(extractors) => extractors.iter().cloned().try_rfold(
            Box::new(NeutralExtractor::<ReqBody>::new()) as RequestExtractor<ReqBody>,
            |inner, item| item.into_extractors(inner),
        ),
    }
}

fn build_endpoint<ReqBody, ResBody>(
    request: MaybeUndefined<Request>,
    response: MaybeUndefined<Response>,
    extractors: MaybeUndefined<Vec<Extractor>>,
    policy: PolicyConfig,
) -> Result<Endpoint<ReqBody, ResBody>, ConfigError>
where
    ReqBody: hyper::body::Body + Send + Unpin + Debug + 'static,
    ReqBody::Error: Debug + Send,
    ReqBody::Data: Send,
    ResBody: hyper::body::Body + Send + Unpin + 'static,
    ResBody::Error: Debug + Send,
    ResBody::Data: Send,
{
    let extractors = Arc::new(build_extractors::<ReqBody>(&extractors)?);
    let response_predicates = Arc::new(match response {
        MaybeUndefined::Value(response) => response.into_predicates()?,
        MaybeUndefined::Null => {
            Box::new(NeutralResponsePredicate::<ResBody>::new()) as ResponsePredicate<ResBody>
        }
        MaybeUndefined::Undefined => {
            Box::new(NeutralResponsePredicate::<ResBody>::new().status_code(StatusCode::OK))
                as ResponsePredicate<ResBody>
        }
    });
    let request_predicates = Arc::new(match request {
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
        policy,
    })
}
