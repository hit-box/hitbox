//! Cache configuration types and implementations.
//!
//! Provides [`Config`] (single configuration) and [`SelectiveConfig`] (multi-config routing),
//! both implementing [`CacheConfigs`] for use with cache services.

use std::sync::Arc;

use hitbox_core::Extractor;
pub use hitbox_core::config::{CacheConfig, CacheConfigs};
use hitbox_core::predicate::Predicate;
use hitbox_core::tag::{NeutralTagExtractor, TagExtractor};

use crate::policy::PolicyConfig;

/// Boxed predicate for dynamic dispatch.
pub type BoxPredicate<R> = Box<dyn Predicate<Subject = R> + Send + Sync>;

/// Boxed extractor for dynamic dispatch.
pub type BoxExtractor<Req> = Box<dyn Extractor<Subject = Req> + Send + Sync>;

/// Boxed tag extractor for dynamic dispatch.
pub type BoxTagExtractor<S> = Box<dyn TagExtractor<Subject = S> + Send + Sync>;

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
/// # let _: Config<
/// #     Neutral<String>,
/// #     Neutral<String>,
/// #     FixedKeyExtractor,
/// #     hitbox_core::tag::NeutralTagExtractor<String>,
/// #     hitbox_core::tag::NeutralTagExtractor<String>,
/// # > = config;
/// ```
pub struct Config<ReqPred, ResPred, Ext, ReqTagExt, ResTagExt> {
    request_predicate: Arc<ReqPred>,
    response_predicate: Arc<ResPred>,
    extractor: Arc<Ext>,
    request_tag_extractors: Arc<ReqTagExt>,
    response_tag_extractors: Arc<ResTagExt>,
    policy: Arc<PolicyConfig>,
}

impl<ReqPred, ResPred, Ext, ReqTagExt, ResTagExt> Clone
    for Config<ReqPred, ResPred, Ext, ReqTagExt, ResTagExt>
{
    fn clone(&self) -> Self {
        Self {
            request_predicate: Arc::clone(&self.request_predicate),
            response_predicate: Arc::clone(&self.response_predicate),
            extractor: Arc::clone(&self.extractor),
            request_tag_extractors: Arc::clone(&self.request_tag_extractors),
            response_tag_extractors: Arc::clone(&self.response_tag_extractors),
            policy: Arc::clone(&self.policy),
        }
    }
}

impl<ReqPred, ResPred, Ext, ReqTagExt, ResTagExt> std::fmt::Debug
    for Config<ReqPred, ResPred, Ext, ReqTagExt, ResTagExt>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("request_predicate", &"...")
            .field("response_predicate", &"...")
            .field("extractor", &"...")
            .field("request_tag_extractors", &"...")
            .field("response_tag_extractors", &"...")
            .field("policy", &self.policy)
            .finish()
    }
}

impl<Req, Res, ReqPred, ResPred, Ext, ReqTagExt, ResTagExt> CacheConfig<Req, Res>
    for Config<ReqPred, ResPred, Ext, ReqTagExt, ResTagExt>
where
    Req: Send,
    Res: Send,
    ReqPred: Predicate<Subject = Req> + Send + Sync + 'static,
    ResPred: Predicate<Subject = Res> + Send + Sync + 'static,
    Ext: Extractor<Subject = Req> + Send + Sync + 'static,
    ReqTagExt: TagExtractor<Subject = Req> + Send + Sync + 'static,
    ResTagExt: TagExtractor<Subject = Res> + Send + Sync + 'static,
{
    type RequestPredicate = Arc<ReqPred>;
    type ResponsePredicate = Arc<ResPred>;
    type Extractor = Arc<Ext>;
    type RequestTagExtractor = Arc<ReqTagExt>;
    type ResponseTagExtractor = Arc<ResTagExt>;

    fn request_predicates(&self) -> Self::RequestPredicate {
        Arc::clone(&self.request_predicate)
    }

    fn response_predicates(&self) -> Self::ResponsePredicate {
        Arc::clone(&self.response_predicate)
    }

    fn extractors(&self) -> Self::Extractor {
        Arc::clone(&self.extractor)
    }

    fn request_tag_extractors(&self) -> Self::RequestTagExtractor {
        Arc::clone(&self.request_tag_extractors)
    }

    fn response_tag_extractors(&self) -> Self::ResponseTagExtractor {
        Arc::clone(&self.response_tag_extractors)
    }

    fn policy(&self) -> Arc<PolicyConfig> {
        Arc::clone(&self.policy)
    }
}

impl<Req, Res, ReqPred, ResPred, Ext, ReqTagExt, ResTagExt> CacheConfigs<Req, Res>
    for Config<ReqPred, ResPred, Ext, ReqTagExt, ResTagExt>
where
    Req: Send,
    Res: Send,
    ReqPred: Predicate<Subject = Req> + Send + Sync + 'static,
    ResPred: Predicate<Subject = Res> + Send + Sync + 'static,
    Ext: Extractor<Subject = Req> + Send + Sync + 'static,
    ReqTagExt: TagExtractor<Subject = Req> + Send + Sync + 'static,
    ResTagExt: TagExtractor<Subject = Res> + Send + Sync + 'static,
{
    type Config = Self;

    fn configs(&self) -> &[Self::Config] {
        std::slice::from_ref(self)
    }
}

// =============================================================================
// SelectiveConfig
// =============================================================================

/// Multi-config container for selective caching.
///
/// Holds multiple [`CacheConfig`] instances evaluated in order by
/// [`SelectiveCacheFuture`](crate::fsm::SelectiveCacheFuture).
/// First matching configuration wins.
#[derive(Debug, Clone)]
pub struct SelectiveConfig<C> {
    configs: Vec<C>,
}

impl<C> SelectiveConfig<C> {
    /// Create a new selective config from a vector of configurations.
    pub fn new(configs: Vec<C>) -> Self {
        Self { configs }
    }
}

impl<Req, Res, C> CacheConfigs<Req, Res> for SelectiveConfig<C>
where
    C: CacheConfig<Req, Res>,
{
    type Config = C;

    fn configs(&self) -> &[C] {
        &self.configs
    }
}

// =============================================================================
// Builder
// =============================================================================

/// Builder for [`Config`].
///
/// Use [`Config::builder()`] to create a new builder.
pub struct ConfigBuilder<ReqPred, ResPred, Ext, ReqTagExt, ResTagExt> {
    request_predicate: ReqPred,
    response_predicate: ResPred,
    extractor: Ext,
    request_tag_extractors: ReqTagExt,
    response_tag_extractors: ResTagExt,
    policy: PolicyConfig,
}

/// Marker type for unset builder fields.
///
/// This type is used in the typestate pattern for `ConfigBuilder`.
/// When you see `NotSet` in a compiler error, it means you haven't called
/// the corresponding builder method yet.
pub struct NotSet;

/// Trait for converting builder tag-extractor slots into a concrete tag extractor.
///
/// This implements option (d) for default tag extractors: when the builder
/// reaches `.build()`, the `Subject` type is inferred from the predicate's
/// associated type, and `NotSet` is materialized as `NeutralTagExtractor<Subject>`.
/// Real tag extractors pass through unchanged.
pub trait MaybeTagExtractor<Subject> {
    /// The resolved tag extractor type.
    type Output: TagExtractor<Subject = Subject> + Send + Sync + 'static;
    /// Convert the builder slot into the concrete tag extractor.
    fn into_tag_extractor(self) -> Self::Output;
}

impl<Subject> MaybeTagExtractor<Subject> for NotSet
where
    Subject: Send + 'static,
{
    type Output = NeutralTagExtractor<Subject>;
    fn into_tag_extractor(self) -> Self::Output {
        NeutralTagExtractor::default()
    }
}

impl<T, Subject> MaybeTagExtractor<Subject> for T
where
    T: TagExtractor<Subject = Subject> + Send + Sync + 'static,
{
    type Output = T;
    fn into_tag_extractor(self) -> Self::Output {
        self
    }
}

impl Config<NotSet, NotSet, NotSet, NotSet, NotSet> {
    /// Creates a new [`ConfigBuilder`].
    pub fn builder() -> ConfigBuilder<NotSet, NotSet, NotSet, NotSet, NotSet> {
        ConfigBuilder::new()
    }
}

impl ConfigBuilder<NotSet, NotSet, NotSet, NotSet, NotSet> {
    /// Creates a new builder with no fields set.
    pub fn new() -> Self {
        Self {
            request_predicate: NotSet,
            response_predicate: NotSet,
            extractor: NotSet,
            request_tag_extractors: NotSet,
            response_tag_extractors: NotSet,
            policy: PolicyConfig::default(),
        }
    }
}

impl Default for ConfigBuilder<NotSet, NotSet, NotSet, NotSet, NotSet> {
    fn default() -> Self {
        Self::new()
    }
}

impl<ReqPred, ResPred, Ext, ReqTagExt, ResTagExt>
    ConfigBuilder<ReqPred, ResPred, Ext, ReqTagExt, ResTagExt>
{
    /// Sets the request predicate.
    pub fn request_predicate<NewReqPred>(
        self,
        predicate: NewReqPred,
    ) -> ConfigBuilder<NewReqPred, ResPred, Ext, ReqTagExt, ResTagExt> {
        ConfigBuilder {
            request_predicate: predicate,
            response_predicate: self.response_predicate,
            extractor: self.extractor,
            request_tag_extractors: self.request_tag_extractors,
            response_tag_extractors: self.response_tag_extractors,
            policy: self.policy,
        }
    }

    /// Sets the response predicate.
    pub fn response_predicate<NewResPred>(
        self,
        predicate: NewResPred,
    ) -> ConfigBuilder<ReqPred, NewResPred, Ext, ReqTagExt, ResTagExt> {
        ConfigBuilder {
            request_predicate: self.request_predicate,
            response_predicate: predicate,
            extractor: self.extractor,
            request_tag_extractors: self.request_tag_extractors,
            response_tag_extractors: self.response_tag_extractors,
            policy: self.policy,
        }
    }

    /// Sets the cache key extractor.
    pub fn extractor<NewExt>(
        self,
        extractor: NewExt,
    ) -> ConfigBuilder<ReqPred, ResPred, NewExt, ReqTagExt, ResTagExt> {
        ConfigBuilder {
            request_predicate: self.request_predicate,
            response_predicate: self.response_predicate,
            extractor,
            request_tag_extractors: self.request_tag_extractors,
            response_tag_extractors: self.response_tag_extractors,
            policy: self.policy,
        }
    }

    /// Sets the request tag extractor (for request-side cache invalidation).
    pub fn request_tags<NewReqTagExt>(
        self,
        tag_extractor: NewReqTagExt,
    ) -> ConfigBuilder<ReqPred, ResPred, Ext, NewReqTagExt, ResTagExt> {
        ConfigBuilder {
            request_predicate: self.request_predicate,
            response_predicate: self.response_predicate,
            extractor: self.extractor,
            request_tag_extractors: tag_extractor,
            response_tag_extractors: self.response_tag_extractors,
            policy: self.policy,
        }
    }

    /// Sets the response tag extractor (for response-side cache invalidation).
    pub fn response_tags<NewResTagExt>(
        self,
        tag_extractor: NewResTagExt,
    ) -> ConfigBuilder<ReqPred, ResPred, Ext, ReqTagExt, NewResTagExt> {
        ConfigBuilder {
            request_predicate: self.request_predicate,
            response_predicate: self.response_predicate,
            extractor: self.extractor,
            request_tag_extractors: self.request_tag_extractors,
            response_tag_extractors: tag_extractor,
            policy: self.policy,
        }
    }

    /// Sets the cache policy.
    pub fn policy(self, policy: PolicyConfig) -> Self {
        Self { policy, ..self }
    }
}

impl<ReqPred, ResPred, Ext, ReqTagExt, ResTagExt>
    ConfigBuilder<ReqPred, ResPred, Ext, ReqTagExt, ResTagExt>
where
    ReqPred: Predicate + Send + Sync + 'static,
    ResPred: Predicate + Send + Sync + 'static,
    Ext: Extractor + Send + Sync + 'static,
    ReqTagExt: MaybeTagExtractor<ReqPred::Subject>,
    ResTagExt: MaybeTagExtractor<ResPred::Subject>,
    ReqPred::Subject: Send + 'static,
    ResPred::Subject: Send + 'static,
{
    /// Builds the [`Config`].
    ///
    /// Required fields (`request_predicate`, `response_predicate`, `extractor`) must be set.
    /// Tag extractors default to [`NeutralTagExtractor`] if not configured.
    pub fn build(self) -> Config<ReqPred, ResPred, Ext, ReqTagExt::Output, ResTagExt::Output> {
        Config {
            request_predicate: Arc::new(self.request_predicate),
            response_predicate: Arc::new(self.response_predicate),
            extractor: Arc::new(self.extractor),
            request_tag_extractors: Arc::new(self.request_tag_extractors.into_tag_extractor()),
            response_tag_extractors: Arc::new(self.response_tag_extractors.into_tag_extractor()),
            policy: Arc::new(self.policy),
        }
    }
}
