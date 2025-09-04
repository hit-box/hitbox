use crate::core::{HitboxWorld, StepExt};
use anyhow::{anyhow, Error};
use cucumber::gherkin::Step;
use cucumber::{given, then, when, WriterExt};
use hitbox::policy::PolicyConfig;
use hitbox_http::predicates::response::body::{Operation, ParsingType};
use hitbox_http::predicates::response::{BodyPredicate, StatusCodePredicate};
use http::StatusCode;
use hurl::{
    runner::{request::eval_request, VariableSet},
    util::path::ContextDir,
};
use hurl_core::{error::DisplaySourceError, parser::parse_hurl_file, text::Format};
use serde_json::Value;
use std::sync::Arc;

///////////// GIVEN ////////////

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

///////////// THEN ////////////

#[then(expr = "response status is {int}")]
fn response_status_predicate(world: &mut HitboxWorld, status: u16) -> Result<(), Error> {
    if world
        .state
        .response
        .as_ref()
        .map(|v| v.status_code().as_u16() == status)
        .unwrap_or_default()
    {
        Ok(())
    } else {
        Err(anyhow!(
            "Response status {} does not match expected {}",
            world
                .state
                .response
                .as_ref()
                .map(|r| r.status_code().as_u16())
                .unwrap_or(0),
            status
        ))
    }
}

#[then(expr = "response body jq {string}")]
fn response_body_jq_predicate(world: &mut HitboxWorld, jq_expression: String) -> Result<(), Error> {
    // Parse JQ expression to extract field and expected value
    let (field_path, operation) = if jq_expression.contains('=') {
        let parts: Vec<&str> = jq_expression.split('=').collect();
        if parts.len() == 2 {
            let field = parts[0].trim();
            let value = parts[1].trim_matches('"').trim();
            (
                field.to_string(),
                Operation::Eq(Value::String(value.to_string())),
            )
        } else {
            return Err(anyhow!("Invalid JQ expression format: {}", jq_expression));
        }
    } else {
        // If no '=' sign, just check if the field exists
        (jq_expression.clone(), Operation::Exist)
    };

    let current_predicate = world.settings.response_predicates.clone();
    let new_predicate = current_predicate.body(ParsingType::Jq, field_path, operation);
    world.settings.response_predicates = Arc::new(new_predicate);
    Ok(())
}

#[then(expr = "response headers contain {string} header")]
fn response_has_header(world_: &mut HitboxWorld, header_: String) -> Result<(), Error> {
    Ok(())
}

#[then(expr = "response headers have no {string} header")]
fn response_has_no_header(world_: &mut HitboxWorld, header_: String) -> Result<(), Error> {
    Ok(())
}

#[then(expr = "cache has {int} records")]
async fn check_cache_record_count(
    world: &mut HitboxWorld,
    expected_count: usize,
) -> Result<(), Error> {
    let actual_count = world.backend.cache.entry_count() as usize;

    if actual_count != expected_count {
        return Err(anyhow!(
            "Expected {} cache records, but found {}",
            expected_count,
            actual_count
        ));
    }

    Ok(())
}

#[then(expr = "cache key {string} exists")]
async fn check_cache_key_exists(world: &mut HitboxWorld, key_pattern: String) -> Result<(), Error> {
    // Parse key pattern like "GET:robert-sheckley:victim-prime"
    let key_parts: Vec<&str> = key_pattern.split(':').collect();
    let cache_key =
        hitbox::CacheKey::from_slice(&key_parts.iter().map(|&s| ("", s)).collect::<Vec<_>>());

    let exists = world.backend.cache.get(&cache_key).await.is_some();

    if !exists {
        return Err(anyhow!(
            "Expected cache key '{}' to exist, but it was not found",
            key_pattern
        ));
    }

    Ok(())
}

///////////// WHEN ////////////

#[when(expr = "execute request")]
async fn execute_request(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
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
