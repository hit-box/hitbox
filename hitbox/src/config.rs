//! Cache configuration trait and type aliases.
//!
//! `CacheConfig` unifies request filtering, response filtering, key extraction,
//! and TTL policy into a single configuration object per endpoint.

use std::sync::Arc;

use async_trait::async_trait;

use crate::Extractor;
use crate::policy::PolicyConfig;
use crate::predicate::{Predicate, PredicateResult};

/// Boxed predicate for dynamic dispatch.
pub type BoxPredicate<R> = Box<dyn Predicate<Subject = R> + Send + Sync>;

/// Boxed extractor for dynamic dispatch.
pub type BoxExtractor<Req> = Box<dyn Extractor<Subject = Req> + Send + Sync>;

/// Trait for cache configuration.
///
/// Provides predicates for determining cacheability, extractors for generating
/// cache keys, and policy for TTL/staleness behavior.
pub trait CacheConfig<Req, Res> {
    /// Predicate type for filtering requests.
    type RequestPredicate: Predicate<Subject = Req> + Send + Sync + 'static;
    /// Predicate type for filtering responses.
    type ResponsePredicate: Predicate<Subject = Res> + Send + Sync + 'static;
    /// Extractor type for generating cache keys.
    type Extractor: Extractor<Subject = Req> + Send + Sync + 'static;

    /// Returns predicates that decide if a request should be cached.
    fn request_predicates(&self) -> Self::RequestPredicate;
    /// Returns predicates that decide if a response should be cached.
    fn response_predicates(&self) -> Self::ResponsePredicate;
    /// Returns extractors that generate cache keys from requests.
    fn extractors(&self) -> Self::Extractor;
    /// Returns TTL and behavior policy for cached entries.
    fn policy(&self) -> &PolicyConfig;
}

/// Route selection result for multi-configuration cache layers.
#[derive(Debug)]
pub enum RouteMatch<Req, ReqPred, ResPred, Ext> {
    /// A configuration matched this request.
    Matched {
        /// Request value after predicate evaluation.
        request: Req,
        /// Request predicates for the selected configuration.
        request_predicates: ReqPred,
        /// Response predicates for the selected configuration.
        response_predicates: ResPred,
        /// Cache key extractors for the selected configuration.
        extractors: Ext,
        /// Cache policy for the selected configuration.
        policy: PolicyConfig,
    },
    /// No configuration matched this request.
    Miss(Req),
}

/// Trait for selecting cache configuration per request.
///
/// This supports both single-config and multi-config cache layers.
#[async_trait]
pub trait CacheConfigRouter<Req, Res>: Send + Sync {
    /// Predicate type for filtering requests.
    type RequestPredicate: Predicate<Subject = Req> + Send + Sync + 'static;
    /// Predicate type for filtering responses.
    type ResponsePredicate: Predicate<Subject = Res> + Send + Sync + 'static;
    /// Extractor type for generating cache keys.
    type Extractor: Extractor<Subject = Req> + Send + Sync + 'static;

    /// Selects the configuration to use for a request.
    async fn route(
        &self,
        request: Req,
    ) -> RouteMatch<Req, Self::RequestPredicate, Self::ResponsePredicate, Self::Extractor>;
}

/// Multi-configuration container for a single cache layer.
///
/// Configurations are evaluated in order. The first request predicate that
/// returns `Cacheable` is selected.
#[derive(Debug, Clone, Default)]
pub struct MultiConfig<C> {
    configs: Vec<C>,
}

impl<C> MultiConfig<C> {
    /// Creates a multi-config container from an ordered list.
    pub fn new(configs: Vec<C>) -> Self {
        Self { configs }
    }

    /// Adds a configuration to the end of the routing list.
    pub fn push(&mut self, config: C) {
        self.configs.push(config);
    }

    /// Returns the ordered configurations.
    pub fn configs(&self) -> &[C] {
        &self.configs
    }

    /// Number of configurations.
    pub fn len(&self) -> usize {
        self.configs.len()
    }

    /// Returns true when no configurations are present.
    pub fn is_empty(&self) -> bool {
        self.configs.is_empty()
    }

    /// Consumes the container and returns the ordered configurations.
    pub fn into_configs(self) -> Vec<C> {
        self.configs
    }
}

#[async_trait]
impl<Req, Res, C> CacheConfigRouter<Req, Res> for MultiConfig<C>
where
    Req: Send + 'static,
    Res: Send + 'static,
    C: CacheConfig<Req, Res> + Send + Sync,
{
    type RequestPredicate = C::RequestPredicate;
    type ResponsePredicate = C::ResponsePredicate;
    type Extractor = C::Extractor;

    async fn route(
        &self,
        request: Req,
    ) -> RouteMatch<Req, Self::RequestPredicate, Self::ResponsePredicate, Self::Extractor> {
        let mut request = request;

        for config in &self.configs {
            let request_predicates = config.request_predicates();
            match request_predicates.check(request).await {
                PredicateResult::Cacheable(request) => {
                    return RouteMatch::Matched {
                        request,
                        request_predicates,
                        response_predicates: config.response_predicates(),
                        extractors: config.extractors(),
                        policy: config.policy().clone(),
                    };
                }
                PredicateResult::NonCacheable(next_request) => {
                    request = next_request;
                }
            }
        }

        RouteMatch::Miss(request)
    }
}

#[async_trait]
impl<Req, Res, ReqPred, ResPred, Ext> CacheConfigRouter<Req, Res> for Config<ReqPred, ResPred, Ext>
where
    Req: Send + 'static,
    Res: Send + 'static,
    ReqPred: Predicate<Subject = Req> + Send + Sync + 'static,
    ResPred: Predicate<Subject = Res> + Send + Sync + 'static,
    Ext: Extractor<Subject = Req> + Send + Sync + 'static,
{
    type RequestPredicate = Arc<ReqPred>;
    type ResponsePredicate = Arc<ResPred>;
    type Extractor = Arc<Ext>;

    async fn route(
        &self,
        request: Req,
    ) -> RouteMatch<Req, Self::RequestPredicate, Self::ResponsePredicate, Self::Extractor> {
        RouteMatch::Matched {
            request,
            request_predicates: Arc::clone(&self.request_predicate),
            response_predicates: Arc::clone(&self.response_predicate),
            extractors: Arc::clone(&self.extractor),
            policy: self.policy.clone(),
        }
    }
}

#[async_trait]
impl<Req, Res, T> CacheConfigRouter<Req, Res> for Arc<T>
where
    Req: Send + 'static,
    Res: Send + 'static,
    T: CacheConfig<Req, Res> + Send + Sync + ?Sized,
{
    type RequestPredicate = T::RequestPredicate;
    type ResponsePredicate = T::ResponsePredicate;
    type Extractor = T::Extractor;

    async fn route(
        &self,
        request: Req,
    ) -> RouteMatch<Req, Self::RequestPredicate, Self::ResponsePredicate, Self::Extractor> {
        RouteMatch::Matched {
            request,
            request_predicates: self.as_ref().request_predicates(),
            response_predicates: self.as_ref().response_predicates(),
            extractors: self.as_ref().extractors(),
            policy: self.as_ref().policy().clone(),
        }
    }
}

#[async_trait]
impl<Req, Res, T> CacheConfigRouter<Req, Res> for Box<T>
where
    Req: Send + 'static,
    Res: Send + 'static,
    T: CacheConfig<Req, Res> + Send + Sync + ?Sized,
{
    type RequestPredicate = T::RequestPredicate;
    type ResponsePredicate = T::ResponsePredicate;
    type Extractor = T::Extractor;

    async fn route(
        &self,
        request: Req,
    ) -> RouteMatch<Req, Self::RequestPredicate, Self::ResponsePredicate, Self::Extractor> {
        RouteMatch::Matched {
            request,
            request_predicates: self.as_ref().request_predicates(),
            response_predicates: self.as_ref().response_predicates(),
            extractors: self.as_ref().extractors(),
            policy: self.as_ref().policy().clone(),
        }
    }
}

/// Generic cache configuration.
///
/// A protocol-agnostic configuration that holds predicates, extractors, and policy.
/// Use this with any protocol (HTTP, gRPC, etc.) by providing appropriate
/// predicates and extractors.
///
/// # Example
///
/// ```
/// use hitbox::{Config, Extractor, KeyParts};
/// use hitbox::policy::PolicyConfig;
/// use hitbox::predicate::Neutral;
/// use std::time::Duration;
/// #
/// # struct FixedKeyExtractor;
/// # #[async_trait::async_trait]
/// # impl Extractor for FixedKeyExtractor {
/// #     type Subject = String;
/// #     async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
/// #         KeyParts::new(subject)
/// #     }
/// # }
///
/// let config = Config::builder()
///     .request_predicate(Neutral::<String>::new())
///     .response_predicate(Neutral::<String>::new())
///     .extractor(FixedKeyExtractor)
///     .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
///     .build();
/// # let _: Config<Neutral<String>, Neutral<String>, FixedKeyExtractor> = config;
/// ```
pub struct Config<ReqPred, ResPred, Ext> {
    request_predicate: Arc<ReqPred>,
    response_predicate: Arc<ResPred>,
    extractor: Arc<Ext>,
    policy: PolicyConfig,
}

impl<ReqPred, ResPred, Ext> Clone for Config<ReqPred, ResPred, Ext> {
    fn clone(&self) -> Self {
        Self {
            request_predicate: Arc::clone(&self.request_predicate),
            response_predicate: Arc::clone(&self.response_predicate),
            extractor: Arc::clone(&self.extractor),
            policy: self.policy.clone(),
        }
    }
}

impl<ReqPred, ResPred, Ext> std::fmt::Debug for Config<ReqPred, ResPred, Ext> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("request_predicate", &"...")
            .field("response_predicate", &"...")
            .field("extractor", &"...")
            .field("policy", &self.policy)
            .finish()
    }
}

impl<Req, Res, ReqPred, ResPred, Ext> CacheConfig<Req, Res> for Config<ReqPred, ResPred, Ext>
where
    Req: Send,
    Res: Send,
    ReqPred: Predicate<Subject = Req> + Send + Sync + 'static,
    ResPred: Predicate<Subject = Res> + Send + Sync + 'static,
    Ext: Extractor<Subject = Req> + Send + Sync + 'static,
{
    type RequestPredicate = Arc<ReqPred>;
    type ResponsePredicate = Arc<ResPred>;
    type Extractor = Arc<Ext>;

    fn request_predicates(&self) -> Self::RequestPredicate {
        Arc::clone(&self.request_predicate)
    }

    fn response_predicates(&self) -> Self::ResponsePredicate {
        Arc::clone(&self.response_predicate)
    }

    fn extractors(&self) -> Self::Extractor {
        Arc::clone(&self.extractor)
    }

    fn policy(&self) -> &PolicyConfig {
        &self.policy
    }
}

/// Builder for [`Config`].
///
/// Use [`Config::builder()`] to create a new builder.
pub struct ConfigBuilder<ReqPred, ResPred, Ext> {
    request_predicate: ReqPred,
    response_predicate: ResPred,
    extractor: Ext,
    policy: PolicyConfig,
}

/// Marker type for unset builder fields.
///
/// This type is used in the typestate pattern for `ConfigBuilder`.
/// When you see `NotSet` in a compiler error, it means you haven't called
/// the corresponding builder method yet.
pub struct NotSet;

impl Config<NotSet, NotSet, NotSet> {
    /// Creates a new [`ConfigBuilder`].
    pub fn builder() -> ConfigBuilder<NotSet, NotSet, NotSet> {
        ConfigBuilder::new()
    }
}

impl ConfigBuilder<NotSet, NotSet, NotSet> {
    /// Creates a new builder with no fields set.
    pub fn new() -> Self {
        Self {
            request_predicate: NotSet,
            response_predicate: NotSet,
            extractor: NotSet,
            policy: PolicyConfig::default(),
        }
    }
}

impl Default for ConfigBuilder<NotSet, NotSet, NotSet> {
    fn default() -> Self {
        Self::new()
    }
}

impl<ReqPred, ResPred, Ext> ConfigBuilder<ReqPred, ResPred, Ext> {
    /// Sets the request predicate.
    pub fn request_predicate<NewReqPred>(
        self,
        predicate: NewReqPred,
    ) -> ConfigBuilder<NewReqPred, ResPred, Ext> {
        ConfigBuilder {
            request_predicate: predicate,
            response_predicate: self.response_predicate,
            extractor: self.extractor,
            policy: self.policy,
        }
    }

    /// Sets the response predicate.
    pub fn response_predicate<NewResPred>(
        self,
        predicate: NewResPred,
    ) -> ConfigBuilder<ReqPred, NewResPred, Ext> {
        ConfigBuilder {
            request_predicate: self.request_predicate,
            response_predicate: predicate,
            extractor: self.extractor,
            policy: self.policy,
        }
    }

    /// Sets the cache key extractor.
    pub fn extractor<NewExt>(self, extractor: NewExt) -> ConfigBuilder<ReqPred, ResPred, NewExt> {
        ConfigBuilder {
            request_predicate: self.request_predicate,
            response_predicate: self.response_predicate,
            extractor,
            policy: self.policy,
        }
    }

    /// Sets the cache policy.
    pub fn policy(self, policy: PolicyConfig) -> Self {
        Self { policy, ..self }
    }
}

impl<ReqPred, ResPred, Ext> ConfigBuilder<ReqPred, ResPred, Ext>
where
    ReqPred: Predicate + Send + Sync + 'static,
    ResPred: Predicate + Send + Sync + 'static,
    Ext: Extractor + Send + Sync + 'static,
{
    /// Builds the [`Config`].
    ///
    /// All fields (request_predicate, response_predicate, extractor) must be set
    /// before calling this method.
    pub fn build(self) -> Config<ReqPred, ResPred, Ext> {
        Config {
            request_predicate: Arc::new(self.request_predicate),
            response_predicate: Arc::new(self.response_predicate),
            extractor: Arc::new(self.extractor),
            policy: self.policy,
        }
    }
}

impl<T, Req, Res> CacheConfig<Req, Res> for Arc<T>
where
    T: CacheConfig<Req, Res>,
{
    type RequestPredicate = T::RequestPredicate;
    type ResponsePredicate = T::ResponsePredicate;
    type Extractor = T::Extractor;

    fn request_predicates(&self) -> Self::RequestPredicate {
        self.as_ref().request_predicates()
    }

    fn response_predicates(&self) -> Self::ResponsePredicate {
        self.as_ref().response_predicates()
    }

    fn extractors(&self) -> Self::Extractor {
        self.as_ref().extractors()
    }

    fn policy(&self) -> &PolicyConfig {
        self.as_ref().policy()
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use crate::{Extractor, KeyParts, policy::PolicyConfig, predicate::PredicateResult};

    use super::{CacheConfigRouter, Config, MultiConfig, RouteMatch};

    #[derive(Clone)]
    struct MatchRequest {
        expected: u32,
    }

    #[async_trait]
    impl crate::Predicate for MatchRequest {
        type Subject = u32;

        async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject> {
            if subject == self.expected {
                PredicateResult::Cacheable(subject)
            } else {
                PredicateResult::NonCacheable(subject)
            }
        }
    }

    #[derive(Clone)]
    struct AlwaysResponse;

    #[async_trait]
    impl crate::Predicate for AlwaysResponse {
        type Subject = u32;

        async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject> {
            PredicateResult::Cacheable(subject)
        }
    }

    #[derive(Clone)]
    struct IdentityExtractor;

    #[async_trait]
    impl Extractor for IdentityExtractor {
        type Subject = u32;

        async fn get(&self, subject: Self::Subject) -> KeyParts<Self::Subject> {
            KeyParts::new(subject)
        }
    }

    #[tokio::test]
    async fn single_config_router_always_matches() {
        let policy = PolicyConfig::disabled();
        let config = Config::builder()
            .request_predicate(MatchRequest { expected: 7 })
            .response_predicate(AlwaysResponse)
            .extractor(IdentityExtractor)
            .policy(policy.clone())
            .build();

        let route = config.route(42).await;

        match route {
            RouteMatch::Matched {
                request,
                policy: selected_policy,
                ..
            } => {
                assert_eq!(request, 42);
                assert_eq!(selected_policy, policy);
            }
            RouteMatch::Miss(_) => panic!("single config should always route"),
        }
    }

    #[tokio::test]
    async fn multi_config_uses_first_matching_route() {
        let first_policy = PolicyConfig::builder()
            .ttl(std::time::Duration::from_secs(11))
            .build();
        let second_policy = PolicyConfig::builder()
            .ttl(std::time::Duration::from_secs(22))
            .build();

        let first = Config::builder()
            .request_predicate(MatchRequest { expected: 1 })
            .response_predicate(AlwaysResponse)
            .extractor(IdentityExtractor)
            .policy(first_policy.clone())
            .build();

        let second = Config::builder()
            .request_predicate(MatchRequest { expected: 1 })
            .response_predicate(AlwaysResponse)
            .extractor(IdentityExtractor)
            .policy(second_policy)
            .build();

        let routing = MultiConfig::new(vec![first, second]);

        let route = routing.route(1).await;

        match route {
            RouteMatch::Matched {
                request,
                policy: selected_policy,
                ..
            } => {
                assert_eq!(request, 1);
                assert_eq!(selected_policy, first_policy);
            }
            RouteMatch::Miss(_) => panic!("expected a matched route"),
        }
    }

    #[tokio::test]
    async fn multi_config_returns_miss_when_nothing_matches() {
        let route_one = Config::builder()
            .request_predicate(MatchRequest { expected: 1 })
            .response_predicate(AlwaysResponse)
            .extractor(IdentityExtractor)
            .build();

        let route_two = Config::builder()
            .request_predicate(MatchRequest { expected: 2 })
            .response_predicate(AlwaysResponse)
            .extractor(IdentityExtractor)
            .build();

        let routing = MultiConfig::new(vec![route_one, route_two]);

        let route = routing.route(9).await;

        match route {
            RouteMatch::Miss(request) => assert_eq!(request, 9),
            RouteMatch::Matched { .. } => panic!("expected route miss"),
        }
    }
}
