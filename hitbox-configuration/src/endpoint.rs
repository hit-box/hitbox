use std::{fmt::Debug, sync::Arc};

use hitbox::{
    Extractor, Predicate,
    config::{BoxExtractor, BoxPredicate, CacheConfig},
    policy::PolicyConfig,
};
use hitbox_http::{CacheableHttpRequest, CacheableHttpResponse};

use crate::ConfigEndpoint;

pub type RequestPredicate<ReqBody> = BoxPredicate<CacheableHttpRequest<ReqBody>>;
pub type ResponsePredicate<ResBody> = BoxPredicate<CacheableHttpResponse<ResBody>>;
pub type RequestExtractor<ReqBody> = BoxExtractor<CacheableHttpRequest<ReqBody>>;

pub type ArcRequestPredicate<ReqBody> =
    Arc<dyn Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync>;
pub type ArcResponsePredicate<ResBody> =
    Arc<dyn Predicate<Subject = CacheableHttpResponse<ResBody>> + Send + Sync>;
pub type ArcRequestExtractor<ReqBody> =
    Arc<dyn Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync>;

#[derive(Debug)]
pub struct Endpoint<ReqBody, ResBody>
where
    ReqBody: hyper::body::Body,
    ResBody: hyper::body::Body,
{
    pub request_predicates: ArcRequestPredicate<ReqBody>,
    pub response_predicates: ArcResponsePredicate<ResBody>,
    pub extractors: ArcRequestExtractor<ReqBody>,
    pub policy: PolicyConfig,
}

impl<ReqBody, ResBody> Clone for Endpoint<ReqBody, ResBody>
where
    ReqBody: hyper::body::Body,
    ResBody: hyper::body::Body,
{
    fn clone(&self) -> Self {
        Self {
            request_predicates: Arc::clone(&self.request_predicates),
            response_predicates: Arc::clone(&self.response_predicates),
            extractors: Arc::clone(&self.extractors.clone()),
            policy: self.policy.clone(),
        }
    }
}

impl<ReqBody, ResBody> Default for Endpoint<ReqBody, ResBody>
where
    ReqBody: hyper::body::Body + Send + Debug + 'static,
    ReqBody::Error: Debug + Send,
    ReqBody::Data: Send,
    ResBody: hyper::body::Body + Send + 'static,
    ResBody::Error: Debug + Send,
    ResBody::Data: Send,
{
    fn default() -> Self {
        ConfigEndpoint::default()
            .into_endpoint()
            .expect("Default endpoint configuration should never fail")
    }
}

impl<ReqBody, ResBody> CacheConfig<CacheableHttpRequest<ReqBody>, CacheableHttpResponse<ResBody>>
    for Endpoint<ReqBody, ResBody>
where
    ReqBody: hyper::body::Body + Send + 'static,
    ReqBody::Error: Send,
    ReqBody::Data: Send,
    ResBody: hyper::body::Body + Send + 'static,
    ResBody::Error: Send,
    ResBody::Data: Send,
{
    fn request_predicates(
        &self,
    ) -> impl hitbox::Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync + 'static
    {
        Arc::clone(&self.request_predicates)
    }

    fn response_predicates(
        &self,
    ) -> impl hitbox::Predicate<Subject = CacheableHttpResponse<ResBody>> + Send + Sync + 'static
    {
        Arc::clone(&self.response_predicates)
    }

    fn extractors(
        &self,
    ) -> impl hitbox::Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync + 'static
    {
        Arc::clone(&self.extractors)
    }

    fn policy(&self) -> &PolicyConfig {
        &self.policy
    }
}
