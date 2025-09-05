use std::sync::Arc;

use crate::core::{HitboxWorld, StepExt};
use hitbox_configuration::extractors::{BoxExtractor, Extractor};
use hitbox_configuration::{Request, Response};
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
        .map(serde_yaml::from_str::<PolicyConfig>)
        .transpose()?
        .unwrap_or_default();
    world.settings.policy = policy;
    Ok(())
}

#[given(expr = "request predicates")]
async fn request_predicates(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let config = serde_yaml::from_str::<Request>(
        step.docstring_content()
            .ok_or(anyhow!("Missing predicates configuration"))?
            .as_str(),
    )?;
    let predicates = config.into_predicates();

    world.settings.request_predicates = Arc::new(predicates);
    Ok(())
}

#[given(expr = "response predicates")]
async fn response_predicates(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let config = serde_yaml::from_str::<Response>(
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
    world.settings.response_predicates = Arc::new(predicates);
    Ok(())
}

#[given(expr = "key extractors")]
async fn key_extractors(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    #[derive(Serialize, Deserialize)]
    struct Config(#[serde(with = "serde_yaml::with::singleton_map_recursive")] Vec<Extractor>);
    let config = serde_yaml::from_str::<Config>(
        step.docstring_content()
            .ok_or(anyhow!("Missing extractors configuration"))?
            .as_str(),
    )?;
    let extractors = config.0.into_iter().rfold(
        Box::new(NeutralExtractor::<axum::body::Body>::new()) as BoxExtractor<_>,
        |inner, item| item.into_extractors(inner),
    );
    world.settings.extractors = Arc::new(extractors);
    Ok(())
}
