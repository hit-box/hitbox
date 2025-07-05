use crate::core::{HitboxWorld, StepExt};
use bytes::Bytes;
use hitbox_configuration::extractors::{BoxExtractor, Extractor};
use hitbox_configuration::Request;
use hitbox_http::extractors::{self, NeutralExtractor};
use hitbox_http::predicates::request;

use anyhow::{anyhow, Error};
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use hitbox::policy::PolicyConfig;
use hitbox::CacheableRequest;
use hitbox::Predicate;
use hitbox_http::predicates::request::{
    BodyPredicate, HeaderPredicate, MethodPredicate, ParsingType, PathPredicate, QueryPredicate,
};
use hitbox_http::predicates::{request::body::Operation, NeutralRequestPredicate};
use hitbox_http::CacheableHttpRequest;
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::Full;
use hurl::{
    runner::{request::eval_request, VariableSet},
    util::path::ContextDir,
};
use hurl_core::{error::DisplaySourceError, parser::parse_hurl_file, text::Format};
use serde::{Deserialize, Serialize};

///////////// GIVEN ////////////

#[given(regex = r"hitbox with policy")]
fn hitbox(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
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
    world.settings.request_predicates = predicates;
    Ok(())
}

#[given(expr = "key extractor {string}")]
async fn key_extractor(_world: &mut HitboxWorld, _extractor: String) -> Result<(), Error> {
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
    world.settings.extractors = extractors;
    dbg!(&world);
    Ok(())
}

///////////// WHEN ////////////

#[when(expr = "execute request")]
async fn execute(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let hurl_request = step
        .docstring_content()
        .ok_or_else(|| anyhow!("request not provided"))?;
    let hurl_file = parse_hurl_file(&hurl_request).map_err(|err| {
        anyhow!(
            "hurl request parse error: {}",
            &err.message(&hurl_request.lines().collect::<Vec<_>>())
                .to_string(Format::Ansi)
        )
    })?;
    let variables = VariableSet::new();
    let request = &hurl_file
        .entries
        .first()
        .ok_or_else(|| anyhow!("request not found"))?
        .request;
    let request = eval_request(request, &variables, &ContextDir::default())
        .map_err(|err| anyhow!("hurl request error {:?}", err))?;
    world.execute_request(&request).await?;
    Ok(())
}

///////////// THEN ////////////

#[then(expr = "response status is {int}")]
async fn check_response_status(world: &mut HitboxWorld, status: u16) -> Result<(), Error> {
    match &world.state.response {
        Some(response) => {
            if response.status_code().eq(&status) {
                Ok(())
            } else {
                Err(anyhow!(
                    "expected response status code is {}, received is {}",
                    response.status_code(),
                    status
                ))
            }
        }
        None => Err(anyhow!("request was not executed")),
    }
}

#[then(expr = "cache has records")]
async fn check_cache_backend_state(_world: &mut HitboxWorld) -> Result<(), Error> {
    Ok(())
}
