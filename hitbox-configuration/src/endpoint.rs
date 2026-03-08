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

impl<ReqBody, ResBody> Debug for Endpoint<ReqBody, ResBody>
where
    ReqBody: hyper::body::Body,
    ResBody: hyper::body::Body,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Endpoint")
            .field("request_predicates", &"...")
            .field("response_predicates", &"...")
            .field("extractors", &"...")
            .field("policy", &self.policy)
            .finish()
    }
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
            extractors: Arc::clone(&self.extractors),
            policy: self.policy.clone(),
        }
    }
}

impl<ReqBody, ResBody> Default for Endpoint<ReqBody, ResBody>
where
    ReqBody: hyper::body::Body + Send + Unpin + Debug + 'static,
    ReqBody::Error: Debug + Send,
    ReqBody::Data: Send,
    ResBody: hyper::body::Body + Send + Unpin + 'static,
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
    type RequestPredicate = ArcRequestPredicate<ReqBody>;
    type ResponsePredicate = ArcResponsePredicate<ResBody>;
    type Extractor = ArcRequestExtractor<ReqBody>;

    fn request_predicates(&self) -> Self::RequestPredicate {
        Arc::clone(&self.request_predicates)
    }

    fn response_predicates(&self) -> Self::ResponsePredicate {
        Arc::clone(&self.response_predicates)
    }

    fn extractors(&self) -> Self::Extractor {
        Arc::clone(&self.extractors)
    }

    fn policy(&self) -> &PolicyConfig {
        &self.policy
    }
}

impl<ReqBody, ResBody> Endpoint<ReqBody, ResBody>
where
    ReqBody: hyper::body::Body,
    ResBody: hyper::body::Body,
{
    /// Create a new builder for Endpoint.
    pub fn builder() -> EndpointBuilder<ReqBody, ResBody> {
        EndpointBuilder::new()
    }
}

/// Builder for Endpoint.
pub struct EndpointBuilder<ReqBody, ResBody>
where
    ReqBody: hyper::body::Body,
    ResBody: hyper::body::Body,
{
    request_predicates: Option<ArcRequestPredicate<ReqBody>>,
    response_predicates: Option<ArcResponsePredicate<ResBody>>,
    extractors: Option<ArcRequestExtractor<ReqBody>>,
    policy: PolicyConfig,
}

impl<ReqBody, ResBody> EndpointBuilder<ReqBody, ResBody>
where
    ReqBody: hyper::body::Body,
    ResBody: hyper::body::Body,
{
    /// Create a new builder with default values.
    pub fn new() -> Self {
        Self {
            request_predicates: None,
            response_predicates: None,
            extractors: None,
            policy: PolicyConfig::default(),
        }
    }

    /// Set the request predicates.
    pub fn request_predicate<P>(self, predicate: P) -> Self
    where
        P: Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync + 'static,
    {
        Self {
            request_predicates: Some(Arc::new(predicate)),
            ..self
        }
    }

    /// Set the response predicates.
    pub fn response_predicate<P>(self, predicate: P) -> Self
    where
        P: Predicate<Subject = CacheableHttpResponse<ResBody>> + Send + Sync + 'static,
    {
        Self {
            response_predicates: Some(Arc::new(predicate)),
            ..self
        }
    }

    /// Set the key extractors.
    pub fn extractor<E>(self, extractor: E) -> Self
    where
        E: Extractor<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync + 'static,
    {
        Self {
            extractors: Some(Arc::new(extractor)),
            ..self
        }
    }

    /// Set the cache policy.
    pub fn policy(self, policy: PolicyConfig) -> Self {
        Self { policy, ..self }
    }

    /// Build the Endpoint, using defaults for any unset fields.
    pub fn build(self) -> Endpoint<ReqBody, ResBody>
    where
        ReqBody: hyper::body::Body + Send + Unpin + Debug + 'static,
        ReqBody::Error: Debug + Send,
        ReqBody::Data: Send,
        ResBody: hyper::body::Body + Send + Unpin + 'static,
        ResBody::Error: Debug + Send,
        ResBody::Data: Send,
    {
        let default = Endpoint::<ReqBody, ResBody>::default();
        Endpoint {
            request_predicates: self
                .request_predicates
                .unwrap_or(default.request_predicates),
            response_predicates: self
                .response_predicates
                .unwrap_or(default.response_predicates),
            extractors: self.extractors.unwrap_or(default.extractors),
            policy: self.policy,
        }
    }
}

impl<ReqBody, ResBody> Default for EndpointBuilder<ReqBody, ResBody>
where
    ReqBody: hyper::body::Body,
    ResBody: hyper::body::Body,
{
    fn default() -> Self {
        Self::new()
    }
}
