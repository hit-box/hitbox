use crate::fsm::world::FsmWorld;
use anyhow::{Error, anyhow};
use cucumber::{gherkin::Step, then};
use hitbox_backend::CacheBackend;

// =============================================================================
// Upstream Call Assertions
// =============================================================================

#[then(expr = "upstream should be called {int} time")]
fn upstream_called_once(world: &mut FsmWorld, expected: usize) -> Result<(), Error> {
    let actual = world.results.upstream_call_count();
    if actual != expected {
        return Err(anyhow!(
            "Expected upstream to be called {} time(s), but was called {} time(s)",
            expected,
            actual
        ));
    }
    Ok(())
}

#[then(expr = "upstream should be called {int} times")]
fn upstream_called_times(world: &mut FsmWorld, expected: usize) -> Result<(), Error> {
    let actual = world.results.upstream_call_count();
    if actual != expected {
        return Err(anyhow!(
            "Expected upstream to be called {} time(s), but was called {} time(s)",
            expected,
            actual
        ));
    }
    Ok(())
}

#[then(expr = "upstream should not be called")]
fn upstream_not_called(world: &mut FsmWorld) -> Result<(), Error> {
    let actual = world.results.upstream_call_count();
    if actual != 0 {
        return Err(anyhow!(
            "Expected upstream to not be called, but was called {} time(s)",
            actual
        ));
    }
    Ok(())
}

#[then(expr = "upstream should be called at most {int} times")]
fn upstream_called_at_most(world: &mut FsmWorld, max_expected: usize) -> Result<(), Error> {
    let actual = world.results.upstream_call_count();
    if actual > max_expected {
        return Err(anyhow!(
            "Expected upstream to be called at most {} time(s), but was called {} time(s)",
            max_expected,
            actual
        ));
    }
    Ok(())
}

// =============================================================================
// Response Assertions
// =============================================================================

#[then(expr = "all responses should equal {int}")]
fn all_responses_equal(world: &mut FsmWorld, expected: u32) -> Result<(), Error> {
    if !world.results.all_responses_eq(expected) {
        let actual_values: Vec<_> = world.results.responses.iter().map(|(r, _)| r.0).collect();
        return Err(anyhow!(
            "Expected all responses to equal {}, but got {:?}",
            expected,
            actual_values
        ));
    }
    Ok(())
}

// =============================================================================
// Cache State Assertions
// =============================================================================

#[then(expr = "cache should be empty")]
async fn cache_is_empty(world: &mut FsmWorld) -> Result<(), Error> {
    if !world.cache_is_empty().await {
        return Err(anyhow!(
            "Expected cache to be empty, but it contains a value"
        ));
    }
    Ok(())
}

#[then(expr = "cache should contain value {int}")]
async fn cache_contains(world: &mut FsmWorld, expected: u32) -> Result<(), Error> {
    use crate::fsm::world::SimpleResponse;
    use hitbox_core::{CacheContext, CacheKey};

    // Sync moka's pending tasks to ensure all writes are visible
    world.backend.cache().run_pending_tasks().await;

    let cache_key = CacheKey::from_str("fixed_key", "value");
    let mut ctx = CacheContext::default().boxed();

    let result = world
        .backend
        .get::<SimpleResponse>(&cache_key, &mut ctx)
        .await;

    match &result {
        Ok(Some(cached)) => {
            if *cached.data() != expected {
                return Err(anyhow!(
                    "Expected cache to contain value {}, but it contains {:?}",
                    expected,
                    cached.data()
                ));
            }
            Ok(())
        }
        Ok(None) => Err(anyhow!(
            "Expected cache to contain value {}, but cache is empty",
            expected
        )),
        Err(e) => Err(anyhow!(
            "Expected cache to contain value {}, but cache get failed: {:?}",
            expected,
            e
        )),
    }
}

// =============================================================================
// Cache Status Assertions
// =============================================================================

#[then(expr = "cache status should be {string}")]
fn cache_status(world: &mut FsmWorld, expected_status: String) -> Result<(), Error> {
    // Check the cache status from the first response context
    if let Some((_, context)) = world.results.responses.first() {
        let actual_status = format!("{:?}", context.status);
        // Normalize comparison (Hit vs CacheHit, etc.)
        let matches = match expected_status.as_str() {
            "Hit" => actual_status.contains("Hit"),
            "Miss" => actual_status.contains("Miss"),
            _ => actual_status == expected_status,
        };
        if !matches {
            return Err(anyhow!(
                "Expected cache status '{}', but got '{}'",
                expected_status,
                actual_status
            ));
        }
    } else {
        return Err(anyhow!("No responses available to check cache status"));
    }
    Ok(())
}

// =============================================================================
// Composition L1/L2 Assertions
// =============================================================================

#[then(expr = "L{int} should be empty")]
async fn layer_is_empty(world: &mut FsmWorld, layer: u8) -> Result<(), Error> {
    let is_empty = match layer {
        1 => world.l1_is_empty().await,
        2 => world.l2_is_empty().await,
        _ => return Err(anyhow!("Unknown layer: L{}", layer)),
    };
    if !is_empty {
        return Err(anyhow!(
            "Expected L{} to be empty, but it contains a value",
            layer
        ));
    }
    Ok(())
}

#[then(expr = "L{int} should contain value {int}")]
async fn layer_contains_value(world: &mut FsmWorld, layer: u8, expected: u32) -> Result<(), Error> {
    let contains = match layer {
        1 => world.l1_contains_value(expected).await,
        2 => world.l2_contains_value(expected).await,
        _ => return Err(anyhow!("Unknown layer: L{}", layer)),
    };
    if !contains {
        return Err(anyhow!(
            "Expected L{} to contain value {}, but it doesn't",
            layer,
            expected
        ));
    }
    Ok(())
}

// =============================================================================
// FSM State Assertions
// =============================================================================

/// Parsed state expectation with optional field assertions.
struct StateExpectation<'a> {
    name: &'a str,
    fields: Vec<(&'a str, &'a str)>,
}

/// Parse a state expectation like "PollCache {concurrency.decision = disabled}"
fn parse_state_expectation(input: &str) -> StateExpectation<'_> {
    let input = input.trim();

    if let Some(brace_start) = input.find('{') {
        let name = input[..brace_start].trim();
        let fields_str = input[brace_start + 1..].trim_end_matches('}').trim();

        let fields: Vec<(&str, &str)> = fields_str
            .split(',')
            .filter_map(|field| {
                let parts: Vec<&str> = field.split('=').collect();
                if parts.len() == 2 {
                    Some((parts[0].trim(), parts[1].trim()))
                } else {
                    None
                }
            })
            .collect();

        StateExpectation { name, fields }
    } else {
        StateExpectation {
            name: input,
            fields: vec![],
        }
    }
}

#[then("FSM states should be:")]
fn fsm_states_should_be(world: &mut FsmWorld, step: &Step) -> Result<(), Error> {
    let table = step
        .table
        .as_ref()
        .ok_or_else(|| anyhow!("FSM states step requires a table"))?;

    // Extract expected states from table (single column, no header)
    let expected: Vec<StateExpectation<'_>> = table
        .rows
        .iter()
        .filter_map(|row| row.first().map(|s| parse_state_expectation(s.as_str())))
        .collect();

    // Get actual FSM state spans (filtered to fsm.* prefix, in order)
    let fsm_spans: Vec<_> = world
        .span_collector
        .spans()
        .into_iter()
        .filter(|s| s.name.starts_with("fsm."))
        .collect();
    let actual_states: Vec<&str> = fsm_spans
        .iter()
        .filter_map(|s| s.name.strip_prefix("fsm."))
        .collect();

    // Check state count
    let expected_names: Vec<&str> = expected.iter().map(|e| e.name).collect();
    if expected.len() != actual_states.len() {
        return Err(anyhow!(
            "FSM state count mismatch.\nExpected {} states: {:?}\nActual {} states: {:?}",
            expected.len(),
            expected_names,
            actual_states.len(),
            actual_states
        ));
    }

    // Check each state name and its fields (position-based)
    for (i, (exp, actual_name)) in expected.iter().zip(actual_states.iter()).enumerate() {
        // Check state name
        if exp.name != *actual_name {
            return Err(anyhow!(
                "FSM state mismatch at position {}.\nExpected: {}\nActual: {}\nFull expected: {:?}\nFull actual: {:?}",
                i,
                exp.name,
                actual_name,
                expected_names,
                actual_states
            ));
        }

        // Check field assertions using position-based span lookup
        for (field_name, expected_value) in &exp.fields {
            let actual_span = &fsm_spans[i];
            let actual_value = actual_span
                .fields
                .iter()
                .find(|(k, _)| k == field_name)
                .map(|(_, v)| v.as_str());

            match actual_value {
                Some(value) if value == *expected_value => {}
                Some(value) => {
                    return Err(anyhow!(
                        "FSM state {} (position {}) field '{}' mismatch.\nExpected: {}\nActual: {}",
                        exp.name,
                        i,
                        field_name,
                        expected_value,
                        value
                    ));
                }
                None => {
                    return Err(anyhow!(
                        "FSM state {} (position {}) field '{}' not found.\nExpected value: {}\nAvailable fields: {:?}",
                        exp.name,
                        i,
                        field_name,
                        expected_value,
                        actual_span.fields
                    ));
                }
            }
        }
    }

    Ok(())
}

#[then("FSM states for each request should be:")]
fn fsm_states_per_request(world: &mut FsmWorld, step: &Step) -> Result<(), Error> {
    let table = step
        .table
        .as_ref()
        .ok_or_else(|| anyhow!("FSM states per request step requires a table"))?;

    // Get the number of requests from the header row
    let header = table
        .rows
        .first()
        .ok_or_else(|| anyhow!("Table must have a header row"))?;
    let num_requests = header.len();

    // Parse expected states for each request from table columns
    let mut expected_per_request: Vec<Vec<&str>> = vec![Vec::new(); num_requests];

    for row in table.rows.iter().skip(1) {
        for (col_idx, cell) in row.iter().enumerate() {
            let trimmed = cell.trim();
            if !trimmed.is_empty() {
                expected_per_request[col_idx].push(trimmed);
            }
        }
    }

    // Get actual FSM states per request from span collector
    let actual_per_request = world.span_collector.fsm_states_per_request();

    // Check we have the expected number of requests
    if actual_per_request.len() != num_requests {
        return Err(anyhow!(
            "Request count mismatch. Expected {} requests, but found {}.\nActual request states: {:?}",
            num_requests,
            actual_per_request.len(),
            actual_per_request
                .iter()
                .map(|r| r
                    .iter()
                    .map(|s| s.name.strip_prefix("fsm.").unwrap_or(&s.name))
                    .collect::<Vec<_>>())
                .collect::<Vec<_>>()
        ));
    }

    // Check states for each request
    for (req_idx, (expected, actual)) in expected_per_request
        .iter()
        .zip(actual_per_request.iter())
        .enumerate()
    {
        let actual_names: Vec<&str> = actual
            .iter()
            .filter_map(|s| s.name.strip_prefix("fsm."))
            .collect();

        // Parse each expected state (may have field assertions)
        let expected_parsed: Vec<StateExpectation<'_>> = expected
            .iter()
            .map(|s| parse_state_expectation(s))
            .collect();

        let expected_names: Vec<&str> = expected_parsed.iter().map(|e| e.name).collect();

        if expected_names.len() != actual_names.len() {
            return Err(anyhow!(
                "Request {} state count mismatch.\nExpected {} states: {:?}\nActual {} states: {:?}",
                req_idx + 1,
                expected_names.len(),
                expected_names,
                actual_names.len(),
                actual_names
            ));
        }

        for (state_idx, (exp, actual_name)) in
            expected_parsed.iter().zip(actual_names.iter()).enumerate()
        {
            if exp.name != *actual_name {
                return Err(anyhow!(
                    "Request {} state mismatch at position {}.\nExpected: {}\nActual: {}\nFull expected: {:?}\nFull actual: {:?}",
                    req_idx + 1,
                    state_idx,
                    exp.name,
                    actual_name,
                    expected_names,
                    actual_names
                ));
            }

            // Check field assertions
            for (field_name, expected_value) in &exp.fields {
                let actual_span = &actual[state_idx];
                let actual_value = actual_span
                    .fields
                    .iter()
                    .find(|(k, _)| k == field_name)
                    .map(|(_, v)| v.as_str());

                match actual_value {
                    Some(value) if value == *expected_value => {}
                    Some(value) => {
                        return Err(anyhow!(
                            "Request {} state {} field '{}' mismatch.\nExpected: {}\nActual: {}",
                            req_idx + 1,
                            exp.name,
                            field_name,
                            expected_value,
                            value
                        ));
                    }
                    None => {
                        return Err(anyhow!(
                            "Request {} state {} field '{}' not found.\nExpected value: {}",
                            req_idx + 1,
                            exp.name,
                            field_name,
                            expected_value
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}
