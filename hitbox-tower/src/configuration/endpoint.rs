use crate::configuration::{
    builder::EndpointConfigBuilder, RequestExtractor, RequestPredicate, ResponsePredicate,
};
use crate::CacheConfig;
use hitbox::policy::PolicyConfig;
use hitbox::predicate::Predicate;
use hitbox::Extractor;
use hitbox_http::extractors::NeutralExtractor;
use hitbox_http::extractors::{
    header::HeaderExtractor, method::MethodExtractor, path::PathExtractor, query::QueryExtractor,
};
use hitbox_http::predicates::{
    request::{HeaderPredicate, MethodPredicate, PathPredicate, QueryPredicate},
    response::StatusCodePredicate,
    NeutralRequestPredicate, NeutralResponsePredicate,
};
use hitbox_http::{CacheableHttpRequest, CacheableHttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    pub request_predicates: Vec<RequestPredicate>,
    pub response_predicates: Vec<ResponsePredicate>,
    pub extractors: Vec<RequestExtractor>,
    pub policy: PolicyConfig,
}

impl EndpointConfig {
    pub fn new() -> Self {
        Self {
            request_predicates: Vec::new(),
            response_predicates: Vec::new(),
            extractors: Vec::new(),
            policy: Default::default(),
        }
    }

    pub fn builder() -> EndpointConfigBuilder {
        EndpointConfigBuilder::new()
    }
}

impl CacheConfig for EndpointConfig {
    fn request_predicates<ReqBody>(
        &self,
    ) -> Box<dyn Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync>
    where
        ReqBody: Send + 'static,
    {
        let acc_predicate = Box::new(NeutralRequestPredicate::new());
        //dbg!(&self.request_predicates);
        self.request_predicates
            .iter()
            .rfold(acc_predicate, |inner, predicate| match predicate {
                RequestPredicate::Path { path } => Box::new(inner.path(path.clone())),
                RequestPredicate::Query { key, value } => {
                    Box::new(inner.query(key.clone(), value.clone()))
                }
                RequestPredicate::Header { key, value } => {
                    Box::new(inner.header(key.clone(), value.clone()))
                }
                RequestPredicate::Method { method } => Box::new(inner.method(method.clone())),
            })
    }

    fn response_predicates<ResBody>(
        &self,
    ) -> Box<dyn Predicate<Subject = CacheableHttpResponse<ResBody>> + Send + Sync>
    where
        ResBody: Send + 'static,
    {
        let acc_predicate = Box::new(NeutralResponsePredicate::new());
        self.response_predicates
            .iter()
            .rfold(acc_predicate, |inner, predicate| match predicate {
                ResponsePredicate::StatusCode { code } => Box::new(inner.status_code(*code)),
            })
    }

    fn extractors<ReqBody>(
        &self,
    ) -> Box<dyn Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync>
    where
        ReqBody: Send + 'static,
    {
        let acc_extractors = Box::new(NeutralExtractor::new());
        self.extractors
            .iter()
            .rfold(acc_extractors, |inner, extractor| match extractor {
                RequestExtractor::Path { path } => Box::new(inner.path(path)),
                RequestExtractor::Method => Box::new(inner.method()),
                RequestExtractor::Query { key } => Box::new(inner.query(key.to_string())),
                RequestExtractor::Header { key } => Box::new(inner.header(key.to_string())),
            })
    }

    fn policy(&self) -> PolicyConfig {
        self.policy.clone()
    }
}

impl<C> CacheConfig for Arc<C>
where
    C: CacheConfig,
{
    fn request_predicates<ReqBody>(
        &self,
    ) -> Box<dyn Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync>
    where
        ReqBody: Send + 'static,
    {
        self.as_ref().request_predicates()
    }

    fn response_predicates<ResBody>(
        &self,
    ) -> Box<dyn Predicate<Subject = CacheableHttpResponse<ResBody>> + Send + Sync>
    where
        ResBody: Send + 'static,
    {
        self.as_ref().response_predicates()
    }

    fn extractors<ReqBody>(
        &self,
    ) -> Box<dyn Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync>
    where
        ReqBody: Send + 'static,
    {
        self.as_ref().extractors()
    }

    fn policy(&self) -> PolicyConfig {
        self.as_ref().policy()
    }
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            request_predicates: Vec::new(),
            response_predicates: vec![ResponsePredicate::StatusCode {
                code: http::StatusCode::OK,
            }],
            extractors: vec![
                RequestExtractor::Path {
                    path: String::from("{path}*"),
                },
                RequestExtractor::Method,
            ],
            policy: Default::default(),
        }
    }
}
