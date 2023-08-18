use crate::{
    request_extractor::RequestExtractor, request_predicate::RequestPredicate,
    response_predicate::ResponsePredicate,
};
use hitbox::predicate::Predicate;
use hitbox::Extractor;
use hitbox_http::extractors::NeutralExtractor;
use hitbox_http::extractors::{
    header::HeaderExtractor, method::MethodExtractor, path::PathExtractor, query::QueryExtractor,
};
use hitbox_http::predicates::{
    header::HeaderPredicate,
    method::MethodPredicate,
    path::PathPredicate,
    //body::BodyPredicate,
    query::QueryPredicate,
    status_code::StatusCodePredicate,
    NeutralRequestPredicate,
    NeutralResponsePredicate,
};
use hitbox_http::{CacheableHttpRequest, CacheableHttpResponse};

pub struct EndpointConfigBuilder<RP> {
    pub request_predicates: RP, // TODO: maybe private?
}

impl<ReqBody> Default for EndpointConfigBuilder<NeutralRequestPredicate<ReqBody>> {
    fn default() -> Self {
        EndpointConfigBuilder {
            request_predicates: NeutralRequestPredicate::new(),
        }
    }
}

#[derive(Debug)]
pub struct EndpointConfig {
    pub request_predicates: Vec<RequestPredicate>,
    pub response_predicates: Vec<ResponsePredicate>,
    pub extractors: Vec<RequestExtractor>,
}

impl EndpointConfig {
    pub fn new() -> Self {
        Self {
            request_predicates: Vec::new(),
            response_predicates: Vec::new(),
            extractors: Vec::new(),
        }
    }

    pub fn builder<ReqBody>() -> EndpointConfigBuilder<NeutralRequestPredicate<ReqBody>> {
        EndpointConfigBuilder::default()
    }

    pub(crate) fn request_predicates<ReqBody>(
        &self,
    ) -> Box<dyn Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync>
    where
        ReqBody: Send + 'static,
    {
        let acc_predicate = Box::new(NeutralRequestPredicate::new());
        dbg!(&self.request_predicates);
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

    pub(crate) fn response_predicates<ResBody>(
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

    pub(crate) fn extractors<ReqBody>(
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
        }
    }
}
