use std::{fmt::Debug, num::NonZeroU8, sync::Arc, time::Duration};

use hitbox::policy;
use hitbox_http::{
    extractors::{MethodConfig, NeutralExtractor, method::MethodExtractor, path::PathExtractor},
    predicates::{
        NeutralRequestPredicate, NeutralResponsePredicate, request::MethodPredicate,
        request::method::Operation as MethodOp, response::StatusCodePredicate,
        response::status::Operation as StatusOp,
    },
};
use http::Method;
use serde::{Deserialize, Serialize};

use crate::{
    ConfigError, Request, RequestPredicate, Response, ResponsePredicate,
    endpoint::{Endpoint, RequestExtractor},
    extractors::Extractor,
    types::MaybeUndefined,
};

// =============================================================================
// Serde-enabled policy types for configuration parsing
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Default)]
pub enum StalePolicy {
    #[default]
    Return,
    Revalidate,
    OffloadRevalidate,
}

impl From<StalePolicy> for policy::StalePolicy {
    fn from(s: StalePolicy) -> Self {
        match s {
            StalePolicy::Return => policy::StalePolicy::Return,
            StalePolicy::Revalidate => policy::StalePolicy::Revalidate,
            StalePolicy::OffloadRevalidate => policy::StalePolicy::OffloadRevalidate,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Default)]
pub struct CacheBehaviorPolicy {
    #[serde(default)]
    stale: StalePolicy,
}

impl From<CacheBehaviorPolicy> for policy::CacheBehaviorPolicy {
    fn from(s: CacheBehaviorPolicy) -> Self {
        policy::CacheBehaviorPolicy {
            stale: s.stale.into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct EnabledCacheConfig {
    #[serde(default, with = "humantime_serde")]
    ttl: Option<Duration>,
    #[serde(default, with = "humantime_serde")]
    stale: Option<Duration>,
    #[serde(default)]
    policy: CacheBehaviorPolicy,
    concurrency: Option<NonZeroU8>,
}

impl From<EnabledCacheConfig> for policy::EnabledCacheConfig {
    fn from(s: EnabledCacheConfig) -> Self {
        policy::EnabledCacheConfig {
            ttl: s.ttl,
            stale: s.stale,
            policy: s.policy.into(),
            concurrency: s.concurrency,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum PolicyConfig {
    Enabled(EnabledCacheConfig),
    Disabled,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        PolicyConfig::Enabled(EnabledCacheConfig::default())
    }
}

impl From<PolicyConfig> for policy::PolicyConfig {
    fn from(s: PolicyConfig) -> Self {
        match s {
            PolicyConfig::Enabled(config) => policy::PolicyConfig::Enabled(config.into()),
            PolicyConfig::Disabled => policy::PolicyConfig::Disabled,
        }
    }
}

// =============================================================================
// ConfigEndpoint
// =============================================================================

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
pub struct ConfigEndpoint {
    /// Optional identifier for this endpoint.
    ///
    /// Surfaced in `Debug` output and error messages produced by
    /// [`ConfigEndpoints::into_endpoints`]. Reserved for future use in
    /// tracing spans and metrics labels; not currently consumed by
    /// runtime routing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub request: MaybeUndefined<Request>,
    #[serde(default)]
    pub response: MaybeUndefined<Response>,
    #[serde(default)]
    pub extractors: MaybeUndefined<Vec<Extractor>>,
    pub policy: PolicyConfig,
}

impl ConfigEndpoint {
    pub fn extractors<ReqBody>(&self) -> Result<RequestExtractor<ReqBody>, ConfigError>
    where
        ReqBody: hyper::body::Body + Send + Debug + 'static,
        ReqBody::Error: Debug + Send,
        ReqBody::Data: Send,
    {
        match &self.extractors {
            MaybeUndefined::Null => Ok(Box::new(NeutralExtractor::<ReqBody>::new())),
            MaybeUndefined::Undefined => Ok(Box::new(
                NeutralExtractor::<ReqBody>::new()
                    .method(MethodConfig::new())
                    .path("{path}*"),
            )),
            MaybeUndefined::Value(extractors) => extractors.iter().cloned().try_rfold(
                Box::new(NeutralExtractor::<ReqBody>::new()) as RequestExtractor<ReqBody>,
                |inner, item| item.into_extractors(inner),
            ),
        }
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
        let extractors = Arc::new(self.extractors()?);
        let response_predicates = Arc::new(match self.response {
            MaybeUndefined::Value(response) => response.into_predicates()?,
            MaybeUndefined::Null => {
                Box::new(NeutralResponsePredicate::<ResBody>::new()) as ResponsePredicate<ResBody>
            }
            MaybeUndefined::Undefined => Box::new(
                NeutralResponsePredicate::<ResBody>::new()
                    .status(StatusOp::eq(http::StatusCode::OK)),
            ) as ResponsePredicate<ResBody>,
        });
        let request_predicates = Arc::new(match self.request {
            MaybeUndefined::Value(request) => request.into_predicates()?,
            MaybeUndefined::Null => {
                Box::new(NeutralRequestPredicate::<ReqBody>::new()) as RequestPredicate<ReqBody>
            }
            MaybeUndefined::Undefined => Box::new(
                NeutralRequestPredicate::<ReqBody>::new().method(MethodOp::eq(Method::GET)),
            ) as RequestPredicate<ReqBody>,
        });
        Ok(Endpoint {
            name: self.name,
            extractors,
            request_predicates,
            response_predicates,
            policy: Arc::new(self.policy.into()),
        })
    }
}

// =============================================================================
// ConfigEndpoints
// =============================================================================

/// A list of endpoint configurations for multi-endpoint cache routing.
///
/// Deserializes transparently from a YAML list of endpoint definitions.
/// Each endpoint is evaluated in order; first match wins.
///
/// # Example
///
/// The value parses as a bare YAML list — embed it under whatever key the
/// surrounding configuration uses (for example, `endpoints:` on a larger
/// config struct):
///
/// ```yaml
/// - name: "user-by-id"
///   request:
///     - Method: GET
///     - Path: "/api/users/{id}"
///   policy:
///     Enabled:
///       ttl: 300s
/// - request:
///     - Method: GET
///   policy:
///     Enabled:
///       ttl: 30s
/// ```
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Default)]
#[serde(transparent)]
pub struct ConfigEndpoints {
    pub endpoints: Vec<ConfigEndpoint>,
}

impl ConfigEndpoints {
    /// Number of endpoint configurations in the list.
    pub fn len(&self) -> usize {
        self.endpoints.len()
    }

    /// Whether the list contains no endpoints.
    pub fn is_empty(&self) -> bool {
        self.endpoints.is_empty()
    }

    /// Iterate over the endpoint configurations by reference.
    pub fn iter(&self) -> std::slice::Iter<'_, ConfigEndpoint> {
        self.endpoints.iter()
    }

    /// Convert all endpoint configurations into runtime [`Endpoint`] instances.
    ///
    /// On failure, the returned error is wrapped in
    /// [`ConfigError::EndpointAt`] carrying the zero-based index of the
    /// failing entry and its `name` when present, making it straightforward
    /// to locate a bad entry in a long list.
    pub fn into_endpoints<ReqBody, ResBody>(
        self,
    ) -> Result<Vec<Endpoint<ReqBody, ResBody>>, ConfigError>
    where
        ReqBody: hyper::body::Body + Send + Unpin + Debug + 'static,
        ReqBody::Error: Debug + Send,
        ReqBody::Data: Send,
        ResBody: hyper::body::Body + Send + Unpin + 'static,
        ResBody::Error: Debug + Send,
        ResBody::Data: Send,
    {
        self.endpoints
            .into_iter()
            .enumerate()
            .map(|(index, ce)| {
                let name = ce.name.clone();
                ce.into_endpoint()
                    .map_err(|source| ConfigError::EndpointAt {
                        index,
                        name,
                        source: Box::new(source),
                    })
            })
            .collect()
    }
}

impl IntoIterator for ConfigEndpoints {
    type Item = ConfigEndpoint;
    type IntoIter = std::vec::IntoIter<ConfigEndpoint>;

    fn into_iter(self) -> Self::IntoIter {
        self.endpoints.into_iter()
    }
}

impl<'a> IntoIterator for &'a ConfigEndpoints {
    type Item = &'a ConfigEndpoint;
    type IntoIter = std::slice::Iter<'a, ConfigEndpoint>;

    fn into_iter(self) -> Self::IntoIter {
        self.endpoints.iter()
    }
}

impl FromIterator<ConfigEndpoint> for ConfigEndpoints {
    fn from_iter<T: IntoIterator<Item = ConfigEndpoint>>(iter: T) -> Self {
        Self {
            endpoints: iter.into_iter().collect(),
        }
    }
}
