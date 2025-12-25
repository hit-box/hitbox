//! Default HTTP caching configuration.
//!
//! Provides [`HttpEndpoint`], a minimal configuration that caches all requests
//! and responses with neutral predicates.

use std::fmt::Debug;

use hitbox::config::CacheConfig;
use hitbox::policy::PolicyConfig;

use crate::extractors::NeutralExtractor;
use crate::predicates::{NeutralRequestPredicate, NeutralResponsePredicate};
use crate::{CacheableHttpRequest, CacheableHttpResponse};

/// Default HTTP endpoint configuration for caching.
///
/// This configuration uses neutral predicates (always cacheable) and a neutral
/// extractor (empty cache key). It caches all requests and responses.
///
/// # Why This Type Exists
///
/// `HttpEndpoint` is a non-generic placeholder used in the builder pattern.
/// Unlike `Endpoint<ReqBody, ResBody>` from `hitbox_configuration`, it doesn't
/// require knowing the body types at builder creation time.
///
/// When you call `Cache::builder()`, it uses `HttpEndpoint` as the default config type.
/// When you call `.config(your_config)`, the type is replaced with your actual configuration.
/// This allows the builder to work without specifying body types upfront.
///
/// # Configurable
///
/// - **Policy**: TTL, stale windows, etc. (via the `policy` field)
///
/// # Custom Configuration
///
/// For custom predicates and extractors, use `hitbox_configuration::Endpoint`
/// instead, which provides a builder pattern for full control over caching behavior.
///
/// [`NeutralRequestPredicate`]: crate::predicates::NeutralRequestPredicate
/// [`NeutralResponsePredicate`]: crate::predicates::NeutralResponsePredicate
/// [`NeutralExtractor`]: crate::extractors::NeutralExtractor
#[derive(Debug, Clone, Default)]
pub struct HttpEndpoint {
    /// Cache policy configuration (TTL, stale windows, etc.).
    pub policy: PolicyConfig,
}

impl<ReqBody, ResBody> CacheConfig<CacheableHttpRequest<ReqBody>, CacheableHttpResponse<ResBody>>
    for HttpEndpoint
where
    ReqBody: hyper::body::Body + Send + Debug + 'static,
    ReqBody::Error: Send,
    ReqBody::Data: Send,
    ResBody: hyper::body::Body + Send + 'static,
    ResBody::Error: Send,
    ResBody::Data: Send,
{
    type RequestPredicate = NeutralRequestPredicate<ReqBody>;
    type ResponsePredicate = NeutralResponsePredicate<ResBody>;
    type Extractor = NeutralExtractor<ReqBody>;

    fn request_predicates(&self) -> Self::RequestPredicate {
        NeutralRequestPredicate::new()
    }

    fn response_predicates(&self) -> Self::ResponsePredicate {
        NeutralResponsePredicate::new()
    }

    fn extractors(&self) -> Self::Extractor {
        NeutralExtractor::new()
    }

    fn policy(&self) -> &PolicyConfig {
        &self.policy
    }
}
