use std::sync::Arc;

use crate::core::{HitboxWorld, StepExt};
use crate::time::{MockTime, MockTimeProvider};
use hitbox_configuration::{extractors::Extractor, Request, RequestExtractor, Response};
use hitbox_core::set_mock_time_provider;
use hitbox_http::extractors::NeutralExtractor;

use anyhow::{anyhow, Error};
use cucumber::gherkin::Step;
use cucumber::given;
use hitbox::policy::PolicyConfig;
use serde::{Deserialize, Serialize};

#[given(regex = r"hitbox with policy")]
fn hitbox_with_policy(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let policy = step
        .docstring_content()
        .as_deref()
        .map(serde_saphyr::from_str::<PolicyConfig>)
        .transpose()?
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
    let predicates = config.into_predicates();

    world.config.request_predicates = Arc::new(predicates);
    Ok(())
}

#[given(expr = "response predicates")]
async fn response_predicates(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let config = serde_saphyr::from_str::<Response>(
        step.docstring_content()
            .ok_or(anyhow!("Missing predicates configuration"))?
            .as_str(),
    )
    .inspect_err(|err| {
        use std::error::Error;
        dbg!(&err.source());
        dbg!(err.location());
    })?;
    let predicates = config.into_predicates();
    world.config.response_predicates = Arc::new(predicates);
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
    let extractors = config.0.into_iter().rev().rfold(
        Box::new(NeutralExtractor::<axum::body::Body>::new()) as RequestExtractor<_>,
        |inner, item| item.into_extractors(inner),
    );
    world.config.extractors = Arc::new(extractors);
    Ok(())
}

#[given(expr = "mock time is enabled")]
fn enable_mock_time(world: &mut HitboxWorld) -> Result<(), Error> {
    // Create mock time and provider
    let mock_time = MockTime::new();
    let provider = MockTimeProvider::new(mock_time.clone());

    // Store in world state for scenario access
    world.time_state.mock_time = Some(mock_time);
    world.time_state.mock_provider = Some(provider.clone());

    // Set globally for CacheValue to use
    set_mock_time_provider(Some(Box::new(provider)));

    Ok(())
}

#[given(expr = "mock time is disabled")]
fn disable_mock_time(world: &mut HitboxWorld) -> Result<(), Error> {
    // Clear from world state
    world.time_state.mock_time = None;
    world.time_state.mock_provider = None;

    // Clear global provider
    set_mock_time_provider(None);

    Ok(())
}

#[given(expr = "mock time is reset")]
fn reset_mock_time(world: &mut HitboxWorld) -> Result<(), Error> {
    if let Some(mock_time) = &world.time_state.mock_time {
        mock_time.reset();
        Ok(())
    } else {
        Err(anyhow!("Mock time is not enabled"))
    }
}
