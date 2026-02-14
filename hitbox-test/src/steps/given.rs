use std::sync::Arc;

use crate::core::{BoxExtractor, HitboxWorld, StepExt};
use crate::handler_state::HandlerName;
use hitbox::offload::OffloadManager;
use hitbox_configuration::{Request, Response, extractors::Extractor};
use hitbox_http::extractors::NeutralExtractor;

use anyhow::{Error, anyhow};
use cucumber::gherkin::Step;
use cucumber::given;
use hitbox::policy::PolicyConfig;
use serde::{Deserialize, Serialize};

#[given(regex = r"hitbox with policy")]
fn hitbox_with_policy(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let policy = Arc::new(
        step.docstring_content()
            .as_deref()
            .map(serde_saphyr::from_str::<PolicyConfig>)
            .transpose()?
            .unwrap_or_default(),
    );
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
