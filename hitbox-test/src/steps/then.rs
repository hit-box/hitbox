use crate::core::HitboxWorld;
use crate::handler_state::HandlerName;
use anyhow::{Error, anyhow};
use cucumber::{gherkin::Step, then};
use jaq_core::{
    Ctx, RcIter,
    load::{Arena, File, Loader},
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
fn response_has_header(world: &mut HitboxWorld, header_name: String) -> Result<(), Error> {
    let response = world
        .state
        .response
        .as_ref()
        .ok_or_else(|| anyhow!("No response available"))?;

    let has_header = response.headers().get(&header_name).is_some();

    if !has_header {
        return Err(anyhow!(
            "Expected header '{}' to be present, but it was not found",
            header_name
        ));
    }

    Ok(())
}

#[then(expr = "response headers have no {string} header")]
fn response_has_no_header(world: &mut HitboxWorld, header_name: String) -> Result<(), Error> {
    let response = world
        .state
        .response
        .as_ref()
        .ok_or_else(|| anyhow!("No response available"))?;

    let has_header = response.headers().get(&header_name).is_some();

    if has_header {
        return Err(anyhow!(
            "Expected header '{}' to NOT be present, but it was found",
            header_name
        ));
    }

    Ok(())
}

#[then(expr = "response header {string} is {string}")]
fn response_header_is_correct(
    world: &mut HitboxWorld,
    header_name: String,
    expected_value: String,
) -> Result<(), Error> {
    let response = world
        .state
        .response
        .as_ref()
        .ok_or_else(|| anyhow!("No response available"))?;

    let header_value = response
        .headers()
        .get(&header_name)
        .ok_or_else(|| anyhow!("Header '{}' not found", header_name))?;

    let actual_value = header_value
        .to_str()
        .map_err(|_| anyhow!("Header '{}' contains invalid UTF-8", header_name))?;

    if actual_value != expected_value {
        return Err(anyhow!(
            "Expected header '{}' to have value '{}', but found '{}'",
            header_name,
            expected_value,
            actual_value
        ));
    }

    Ok(())
}

#[then(expr = "response header {string} starts with {string}")]
fn response_header_starts_with(
    world: &mut HitboxWorld,
    header_name: String,
    expected_prefix: String,
) -> Result<(), Error> {
    let response = world
        .state
        .response
        .as_ref()
        .ok_or_else(|| anyhow!("No response available"))?;

    let header_value = response
        .headers()
        .get(&header_name)
        .ok_or_else(|| anyhow!("Header '{}' not found", header_name))?;

    let actual_value = header_value
        .to_str()
        .map_err(|_| anyhow!("Header '{}' contains invalid UTF-8", header_name))?;

    if !actual_value.starts_with(&expected_prefix) {
        return Err(anyhow!(
            "Expected header '{}' to start with '{}', but found '{}'",
            header_name,
            expected_prefix,
            actual_value
        ));
    }

    Ok(())
}

#[then(expr = "backend read was called {int} times with all miss")]
fn backend_read_all_miss(world: &mut HitboxWorld, expected: usize) -> Result<(), Error> {
    let read_count = world.backend.read_count();
    let miss_count = world.backend.read_miss_count();

    if read_count != expected || miss_count != expected {
        return Err(anyhow!(
            "Expected {} reads with all miss, but got {} reads ({} miss, {} hit)",
            expected,
            read_count,
            miss_count,
            world.backend.read_hit_count()
        ));
    }
    Ok(())
}

#[then(expr = "backend write was called {int} times")]
fn backend_write_count(world: &mut HitboxWorld, expected: usize) -> Result<(), Error> {
    let actual = world.backend.write_count();
    if actual != expected {
        return Err(anyhow!(
            "Expected backend write to be called {} times, but was called {} times",
            expected,
            actual
        ));
    }
    Ok(())
}

#[then(expr = "cache has {int} records")]
async fn check_cache_record_count(
    world: &mut HitboxWorld,
    expected_count: usize,
) -> Result<(), Error> {
    let actual_count = world.backend.cache_entry_count();

    if actual_count != expected_count {
        return Err(anyhow!(
            "Expected {} cache records, but found {}",
            expected_count,
            actual_count
        ));
    }

    Ok(())
}

#[then(expr = "all responses should have status {int}")]
fn all_responses_status(world: &mut HitboxWorld, status: u16) -> Result<(), Error> {
    if world.state.responses.is_empty() {
        return Err(anyhow!("No responses available"));
    }

    for (i, response) in world.state.responses.iter().enumerate() {
        let actual = response.status_code().as_u16();
        if actual != status {
            return Err(anyhow!(
                "Response {} has status {}, expected {}",
                i,
                actual,
                status
            ));
        }
    }
    Ok(())
}

#[then(expr = "response headers are")]
fn response_headers_table(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let table = step
        .table
        .as_ref()
        .ok_or_else(|| anyhow!("Expected a table"))?;

    if table.rows.len() != world.state.responses.len() {
        return Err(anyhow!(
            "Expected {} rows but got {}",
            world.state.responses.len(),
            table.rows.len()
        ));
    }

    for (i, (row, response)) in table
        .rows
        .iter()
        .zip(world.state.responses.iter())
        .enumerate()
    {
        let header_name = row
            .first()
            .ok_or_else(|| anyhow!("Row {} missing header name", i))?;
        let expected_value = row
            .get(1)
            .ok_or_else(|| anyhow!("Row {} missing value", i))?;

        let actual_value = response
            .headers()
            .get(header_name)
            .map(|v| v.to_str().unwrap_or(""))
            .unwrap_or("");

        if actual_value != expected_value {
            return Err(anyhow!(
                "Response {}: header '{}' expected '{}', got '{}'",
                i,
                header_name,
                expected_value,
                actual_value
            ));
        }
    }

    Ok(())
}

#[then(expr = "response headers start with")]
fn response_headers_start_with_table(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let table = step
        .table
        .as_ref()
        .ok_or_else(|| anyhow!("Expected a table"))?;

    if table.rows.len() != world.state.responses.len() {
        return Err(anyhow!(
            "Expected {} rows but got {}",
            world.state.responses.len(),
            table.rows.len()
        ));
    }

    for (i, (row, response)) in table
        .rows
        .iter()
        .zip(world.state.responses.iter())
        .enumerate()
    {
        let header_name = row
            .first()
            .ok_or_else(|| anyhow!("Row {} missing header name", i))?;
        let expected_prefix = row
            .get(1)
            .ok_or_else(|| anyhow!("Row {} missing value", i))?;

        let actual_value = response
            .headers()
            .get(header_name)
            .map(|v| v.to_str().unwrap_or(""))
            .unwrap_or("");

        if !actual_value.starts_with(expected_prefix.as_str()) {
            return Err(anyhow!(
                "Response {}: header '{}' expected to start with '{}', got '{}'",
                i,
                header_name,
                expected_prefix,
                actual_value
            ));
        }
    }

    Ok(())
}

#[then(expr = "{word} should be called 1 time")]
fn upstream_called_once(world: &mut HitboxWorld, handler: String) -> Result<(), Error> {
    let handler_name: HandlerName = handler
        .parse()
        .map_err(|_| anyhow!("Unknown handler: {}", handler))?;
    let actual = world.handler_state.get_call_count(handler_name);
    if actual != 1 {
        return Err(anyhow!(
            "Expected {} to be called 1 time, but was called {} times",
            handler,
            actual
        ));
    }
    Ok(())
}

#[then(expr = "{word} should be called {int} times")]
fn upstream_called_times(
    world: &mut HitboxWorld,
    handler: String,
    expected: usize,
) -> Result<(), Error> {
    let handler_name: HandlerName = handler
        .parse()
        .map_err(|_| anyhow!("Unknown handler: {}", handler))?;
    let actual = world.handler_state.get_call_count(handler_name);
    if actual != expected {
        return Err(anyhow!(
            "Expected {} to be called {} times, but was called {} times",
            handler,
            expected,
            actual
        ));
    }
    Ok(())
}

#[then(expr = "cache key exists")]
async fn check_cache_key_exists(world: &mut HitboxWorld, step: &Step) -> Result<(), Error> {
    let cache_key = if let Some(table) = &step.table {
        // Parse from table format (supports duplicate keys, no header row)
        let parts: Vec<hitbox_core::KeyPart> = table
            .rows
            .iter()
            .map(|row| {
                let key = row.first().map(|s| s.as_str()).unwrap_or("");
                let value = row.get(1).map(|s| s.as_str());
                let value = match value {
                    Some("null") | None => None,
                    Some(v) => Some(v.to_string()),
                };
                hitbox_core::KeyPart::new(key, value)
            })
            .collect();
        hitbox_core::CacheKey::new(String::new(), 0, parts)
    } else if let Some(docstring) = &step.docstring {
        // Parse from YAML docstring format
        crate::cache_key::deserialize_debug(docstring.as_bytes())
            .map_err(|e| anyhow!("Failed to parse cache key pattern '{}': {}", docstring, e))?
    } else {
        return Err(anyhow!("Expected table or docstring for cache key"));
    };

    let exists = world.backend.cache.get(&cache_key).is_some();

    if !exists {
        return Err(anyhow!(
            "Expected cache key '{:?}' to exist, but it was not found",
            cache_key
        ));
    }

    Ok(())
}
