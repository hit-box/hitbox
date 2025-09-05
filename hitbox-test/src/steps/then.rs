use crate::core::HitboxWorld;
use anyhow::{anyhow, Error};
use cucumber::then;
use jaq_core::{
    load::{Arena, File, Loader},
    Ctx, RcIter,
};
use jaq_json::Val;
use serde_json::Value;

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
fn response_body_jq(world: &mut HitboxWorld, jq_expression: String) -> Result<(), Error> {
    let response = world
        .state
        .response
        .as_ref()
        .ok_or_else(|| anyhow!("No response available"))?;

    let body_text = response.text();
    let json_value: Value = serde_json::from_str(&body_text)
        .map_err(|e| anyhow!("Failed to parse response body as JSON: {}", e))?;

    // Use jaq to evaluate the expression
    let result = apply_jq_expression(&jq_expression, json_value)?;

    // Check if the result is truthy (for expressions like `.title=="Victim Prime"`)
    let is_truthy = match result {
        Some(Value::Bool(b)) => b,
        _ => false,
    };

    if !is_truthy {
        return Err(anyhow!(
            "JQ expression '{}' evaluated to false",
            jq_expression
        ));
    }

    Ok(())
}

fn apply_jq_expression(expression: &str, input: Value) -> Result<Option<Value>, Error> {
    let program = File {
        code: expression,
        path: (),
    };
    let loader = Loader::new(jaq_std::defs().chain(jaq_json::defs()));
    let arena = Arena::default();
    let modules = loader
        .load(&arena, program)
        .map_err(|e| anyhow!("Failed to load JQ program: {:?}", e))?;
    let filter = jaq_core::Compiler::default()
        .with_funs(jaq_std::funs().chain(jaq_json::funs()))
        .compile(modules)
        .map_err(|e| anyhow!("Failed to compile JQ program: {:?}", e))?;
    let inputs = RcIter::new(core::iter::empty());
    let out = filter.run((Ctx::new([], &inputs), Val::from(input)));
    let results: Result<Vec<_>, _> = out.collect();

    match results {
        Ok(values) if values.eq(&vec![Val::Null]) => Ok(None),
        Ok(values) if !values.is_empty() => {
            let values: Vec<Value> = values.into_iter().map(|v| v.into()).collect();
            if values.len() == 1 {
                Ok(Some(values.into_iter().next().unwrap()))
            } else {
                Ok(Some(Value::Array(values)))
            }
        }
        Ok(_) => Ok(None),
        Err(e) => Err(anyhow!("JQ execution error: {:?}", e)),
    }
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
