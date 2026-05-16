use std::num::NonZeroU8;
use std::sync::Arc;
use std::time::Duration;

use crate::core::{
    BoxExtractor, BoxRequestTagExtractor, BoxResponseTagExtractor, HitboxWorld, StepExt,
};
use crate::handler_state::HandlerName;
use hitbox::offload::OffloadManager;
use hitbox::policy;
use hitbox_configuration::extractors::tag::{
    self as tag_config, RequestTagExtractor, ResponseTagExtractor,
};
use hitbox_configuration::{Request, Response, extractors::Extractor};
use hitbox_http::extractors::NeutralExtractor;

use anyhow::{Error, anyhow};
use cucumber::gherkin::Step;
use cucumber::given;
use serde::{Deserialize, Serialize};

// =============================================================================
// Serde-enabled policy types for YAML deserialization in BDD tests
// =============================================================================

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
enum StalePolicy {
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
enum TagInvalidation {
    #[default]
    Check,
    Skip,
}

impl From<TagInvalidation> for policy::TagInvalidation {
    fn from(t: TagInvalidation) -> Self {
        match t {
            TagInvalidation::Check => policy::TagInvalidation::Check,
            TagInvalidation::Skip => policy::TagInvalidation::Skip,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CacheBehaviorPolicy {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EnabledCacheConfig {
    #[serde(default, with = "humantime_serde")]
    ttl: Option<Duration>,
    #[serde(default, with = "humantime_serde")]
    stale: Option<Duration>,
    #[serde(default)]
    policy: CacheBehaviorPolicy,
    concurrency: Option<NonZeroU8>,
    #[serde(default)]
    tag_invalidation: TagInvalidation,
}

impl Default for EnabledCacheConfig {
    fn default() -> Self {
        Self {
            ttl: Some(Duration::from_secs(5)),
            stale: None,
            policy: CacheBehaviorPolicy::default(),
            concurrency: None,
            tag_invalidation: TagInvalidation::default(),
        }
    }
}

impl From<EnabledCacheConfig> for policy::EnabledCacheConfig {
    fn from(s: EnabledCacheConfig) -> Self {
        policy::EnabledCacheConfig {
            ttl: s.ttl,
            stale: s.stale,
            policy: s.policy.into(),
            concurrency: s.concurrency,
            tag_invalidation: s.tag_invalidation.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum PolicyConfig {
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

#[given(regex = r"hitbox with policy")]
fn hitbox_with_policy(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let policy: policy::PolicyConfig = step
        .docstring_content()
        .as_deref()
        .map(serde_saphyr::from_str::<PolicyConfig>)
        .transpose()?
        .map(Into::into)
        .unwrap_or_default();
    world.config.policy = policy;
    Ok(())
}

#[given(expr = "request predicates")]
async fn request_predicates(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let config = serde_saphyr::from_str::<Request>(
        step.docstring_content()
            .ok_or(anyhow!("Missing predicates configuration"))?
            .as_str(),
    )?;
    let predicates = config.into_predicates()?;

    world.config.request_predicate = Arc::new(predicates);
    Ok(())
}

#[given(expr = "response predicates")]
async fn response_predicates(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let config = serde_saphyr::from_str::<Response>(
        step.docstring_content()
            .ok_or(anyhow!("Missing predicates configuration"))?
            .as_str(),
    )?;
    let predicates = config.into_predicates()?;
    world.config.response_predicate = Arc::new(predicates);
    Ok(())
}

#[given(expr = "key extractors")]
async fn key_extractors(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    #[derive(Serialize, Deserialize)]
    struct Config(Vec<Extractor>);
    let config = serde_saphyr::from_str::<Config>(
        step.docstring_content()
            .ok_or(anyhow!("Missing extractors configuration"))?
            .as_str(),
    )?;
    let extractors = config.0.into_iter().rev().try_rfold(
        Box::new(NeutralExtractor::<axum::body::Body>::new()) as BoxExtractor,
        |inner, item| item.into_extractors(inner),
    )?;
    world.config.extractor = Arc::new(extractors);
    Ok(())
}

#[given(expr = "tags")]
async fn tags(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    #[derive(Serialize, Deserialize, Default)]
    struct Config {
        #[serde(default)]
        request: Vec<RequestTagExtractor>,
        #[serde(default)]
        response: Vec<ResponseTagExtractor>,
    }
    let config = serde_saphyr::from_str::<Config>(
        step.docstring_content()
            .ok_or(anyhow!("Missing tags configuration"))?
            .as_str(),
    )?;

    if !config.request.is_empty() {
        let request_tag_extractor: BoxRequestTagExtractor =
            tag_config::request::build_request_boxed::<axum::body::Body>(config.request)?;
        world.config.request_tag_extractor = Arc::new(request_tag_extractor);
    }
    if !config.response.is_empty() {
        let response_tag_extractor: BoxResponseTagExtractor =
            tag_config::build_response_boxed(config.response);
        world.config.response_tag_extractor = Arc::new(response_tag_extractor);
    }
    Ok(())
}

#[given(expr = "offload revalidation is enabled")]
fn enable_offload_revalidation(world: &mut HitboxWorld) -> Result<(), Error> {
    world.offload_manager = Some(OffloadManager::with_defaults());
    Ok(())
}

#[given(expr = "upstream delay for {word} is {int}ms")]
fn upstream_delay(world: &mut HitboxWorld, handler: String, delay_ms: u64) -> Result<(), Error> {
    let handler_name: HandlerName = handler
        .parse()
        .map_err(|_| anyhow!("Unknown handler: {}", handler))?;
    world.handler_state.set_delay(handler_name, delay_ms);
    Ok(())
}
