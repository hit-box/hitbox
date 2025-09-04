use std::sync::Arc;

use crate::core::{HitboxWorld, StepExt};
use assert_json_diff::{assert_json_eq, assert_json_include};
use axum::body::to_bytes;
use hitbox_configuration::extractors::{BoxExtractor, Extractor};
use hitbox_configuration::{Request, Response};
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
                "received response status code is {}, expected is {}",
                response.status_code(),
                status
            )),
        None => Err(anyhow!("request was not executed")),
    }
}

#[then(expr = "cache has records")]
async fn check_cache_backend_state(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let table = step
        .table
        .as_ref()
        .ok_or_else(|| anyhow!("Expected table with cache records but none found"))?;

    for row in &table.rows {
        let key = parse_key(&row[0])?;
        let cached_body = get_body(world, &key).await?;

        // assert_str_eq!(cached_body, row[1]);
        let out = serde_json::from_str::<serde_json::Value>(&cached_body)?;
        let expected = serde_json::from_str::<serde_json::Value>(&row[1])?;
        // let expected = json!([{"id": "journey-beyond-tomorrow"}, {"id": "victim-prime"}]);
        assert_json_include!(actual: out, expected: expected);
        // if cached_body != row[1] {
        //     return Err(anyhow!(
        //         "Cache body mismatch for key {:?}. Expected: '{}', Found: '{}'",
        //         key,
        //         row[1],
        //         cached_body
        //     ));
        // }
    }
    Ok(())
}

fn parse_key(key_str: &str) -> Result<CacheKey, Error> {
    let key_parts: Result<Vec<_>, _> = key_str
        .split(',')
        .map(|part| {
            let mut key_value = part.split(':');
            match (key_value.next(), key_value.next()) {
                (Some(key), Some(value)) => Ok((key, value)),
                _ => Err(anyhow!(
                    "Invalid key format: '{}'. Expected 'key:value'",
                    part
                )),
            }
        })
        .collect();

    Ok(CacheKey::from_slice(&key_parts?))
}

async fn get_body(world: &mut HitboxWorld, key: &CacheKey) -> Result<String, Error> {
    let value = world
        .backend
        .cache
        .get(key)
        .await
        .ok_or_else(|| anyhow!("Cache missing expected key: {:?}", key))?;

    let cached: SerializableHttpResponse = serde_json::from_slice(&value.data)
        .map_err(|e| anyhow!("Failed to deserialize cached response: {}", e))?;

    let response = CacheableHttpResponse::<axum::body::Body>::from_cached(cached).await;
    let res = response.into_response();

    let bytes = to_bytes(res.into_body(), 100000).await.map_err(|e| {
        anyhow!(
            "Failed to read response body (size > 100k or other error): {}",
            e
        )
    })?;

    String::from_utf8(bytes.to_vec())
        .map_err(|e| anyhow!("Response body is not valid UTF-8: {}", e))
}
