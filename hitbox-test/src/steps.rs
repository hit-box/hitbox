use std::sync::Arc;

use crate::core::{Handler, HitboxWorld, StepExt};
use axum::body::to_bytes;
use hitbox_configuration::extractors::{BoxExtractor, Extractor};
use hitbox_configuration::Request;
use hitbox_http::extractors::NeutralExtractor;

use anyhow::{anyhow, Error};
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use hitbox::policy::PolicyConfig;
use hitbox::CacheKey;
use hitbox::CacheableResponse;
use hitbox_http::{CacheableHttpResponse, SerializableHttpResponse};
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
    world.settings.request_predicates = Arc::new(predicates);
    Ok(())
}

#[given(expr = "handler")]
async fn handler(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let handler_config_content = step
        .docstring_content()
        .ok_or(anyhow!("Missing extractors configuration"))?;
    let handler_config = handler_config_content.as_str();
    let handler = serde_yaml::from_str::<Handler>(handler_config)?;
    world.settings.handler = handler;
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
        Some(response) => response
            .status_code()
            .eq(&status)
            .then_some(())
            .ok_or(anyhow!(
                "expected response status code is {}, received is {}",
                response.status_code(),
                status
            )),
        None => Err(anyhow!("request was not executed")),
    }
}

#[then(expr = "cache has records")]
async fn check_cache_backend_state(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    for row in step.table.as_ref().unwrap().rows.iter() {
        let mut key_parts = vec![];
        let parts: Vec<_> = row[0].split(',').collect();
        for part in parts.into_iter() {
            let keys: Vec<_> = part.split(":").collect();
            key_parts.push((keys[0], keys[1]));
        }
        let key = CacheKey::from_slice(&key_parts);
        let value = world.backend.cache.get(&key).await.unwrap();
        let cached: SerializableHttpResponse = serde_json::from_slice(&value.data).unwrap();
        let response = CacheableHttpResponse::<axum::body::Body>::from_cached(cached).await;
        let res = response.into_response();
        let bytes = to_bytes(res.into_body(), 100000).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(body, row[1]);
    }
    Ok(())
}
