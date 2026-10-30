use crate::fsm::world::{CacheState, FsmConfig, FsmWorld};
use anyhow::{Error, anyhow};
use cucumber::given;
use hitbox::policy::ConcurrencyLimit;
use hitbox_backend::composition::policy::RefillPolicy;

// =============================================================================
// Background/Setup Steps
// =============================================================================

#[given(expr = "upstream response delay is {int}ms")]
fn upstream_delay(world: &mut FsmWorld, delay_ms: u64) -> Result<(), Error> {
    world.upstream_delay_ms = delay_ms;
    Ok(())
}

#[given(expr = "request delay between concurrent requests is {int}ms")]
fn request_delay(world: &mut FsmWorld, delay_ms: u64) -> Result<(), Error> {
    world.request_delay_ms = delay_ms;
    Ok(())
}

// =============================================================================
// Cache Policy Steps
// =============================================================================

#[given(expr = "cache policy is {string}")]
fn cache_policy(world: &mut FsmWorld, policy: String) -> Result<(), Error> {
    match policy.as_str() {
        "Enabled" => world.config.cache_enabled = true,
        "Disabled" => world.config.cache_enabled = false,
        _ => return Err(anyhow::anyhow!("Unknown cache policy: {}", policy)),
    }
    Ok(())
}

// =============================================================================
// Request Cacheability Steps
// =============================================================================

#[given(expr = "request is cacheable")]
fn request_cacheable(world: &mut FsmWorld) -> Result<(), Error> {
    world.config.request_cacheable = true;
    Ok(())
}

#[given(expr = "request is non-cacheable")]
fn request_non_cacheable(world: &mut FsmWorld) -> Result<(), Error> {
    world.config.request_cacheable = false;
    Ok(())
}

// =============================================================================
// Response Cacheability Steps
// =============================================================================

#[given(expr = "response is cacheable")]
fn response_cacheable(world: &mut FsmWorld) -> Result<(), Error> {
    world.config.response_cacheable = true;
    Ok(())
}

#[given(expr = "response is non-cacheable")]
fn response_non_cacheable(world: &mut FsmWorld) -> Result<(), Error> {
    world.config.response_cacheable = false;
    Ok(())
}

// =============================================================================
// Cache State Steps
// =============================================================================

#[given(expr = "cache is empty")]
fn cache_empty(world: &mut FsmWorld) -> Result<(), Error> {
    world.cache_state = CacheState::Empty;
    Ok(())
}

#[given(expr = "cache contains fresh value {int}")]
fn cache_fresh(world: &mut FsmWorld, value: u32) -> Result<(), Error> {
    world.cache_state = CacheState::Fresh(value);
    Ok(())
}

#[given(expr = "cache contains stale value {int}")]
fn cache_stale(world: &mut FsmWorld, value: u32) -> Result<(), Error> {
    world.cache_state = CacheState::Stale(value);
    Ok(())
}

#[given(expr = "cache contains expired value {int}")]
fn cache_expired(world: &mut FsmWorld, value: u32) -> Result<(), Error> {
    world.cache_state = CacheState::Expired(value);
    Ok(())
}

// =============================================================================
// Concurrency Control Steps
// =============================================================================

#[given(expr = "concurrency control is disabled")]
fn concurrency_disabled(world: &mut FsmWorld) -> Result<(), Error> {
    world.config.concurrency = None;
    Ok(())
}

#[given(expr = "concurrency limit is {int}")]
fn concurrency_limit(world: &mut FsmWorld, limit: u8) -> Result<(), Error> {
    let concurrency = ConcurrencyLimit::new(limit)
        .ok_or_else(|| anyhow!("Invalid concurrency limit: {}", limit))?;
    world.config.concurrency = Some(concurrency);
    Ok(())
}

// =============================================================================
// Broadcast Channel Error Simulation Steps (for edge cases)
// Note: These are placeholders - actual implementation would require
// injecting mock concurrency managers
// =============================================================================

#[given(expr = "broadcast channel will be closed")]
fn broadcast_will_close(_world: &mut FsmWorld) -> Result<(), Error> {
    // TODO: This requires injecting a mock concurrency manager
    // that forces channel closed behavior
    Ok(())
}

#[given(expr = "broadcast channel will lag")]
fn broadcast_will_lag(_world: &mut FsmWorld) -> Result<(), Error> {
    // TODO: This requires injecting a mock concurrency manager
    // that forces channel lagged behavior
    Ok(())
}

// =============================================================================
// Composition Backend Steps
// =============================================================================

#[given(expr = "composition backend is enabled")]
fn composition_enabled(world: &mut FsmWorld) -> Result<(), Error> {
    world.composition.enabled = true;
    Ok(())
}

#[given(expr = "refill policy is {string}")]
fn refill_policy(world: &mut FsmWorld, policy: String) -> Result<(), Error> {
    world.composition.refill_policy = match policy.as_str() {
        "Always" => RefillPolicy::Always,
        "Never" => RefillPolicy::Never,
        _ => return Err(anyhow::anyhow!("Unknown refill policy: {}", policy)),
    };
    Ok(())
}

#[given(expr = "L{int} is empty")]
fn layer_empty(world: &mut FsmWorld, layer: u8) -> Result<(), Error> {
    match layer {
        1 => world.composition.l1_state = CacheState::Empty,
        2 => world.composition.l2_state = CacheState::Empty,
        _ => return Err(anyhow::anyhow!("Unknown layer: L{}", layer)),
    }
    Ok(())
}

#[given(expr = "L{int} contains fresh value {int}")]
fn layer_fresh(world: &mut FsmWorld, layer: u8, value: u32) -> Result<(), Error> {
    let state = CacheState::Fresh(value);
    match layer {
        1 => world.composition.l1_state = state,
        2 => world.composition.l2_state = state,
        _ => return Err(anyhow::anyhow!("Unknown layer: L{}", layer)),
    }
    Ok(())
}

#[given(expr = "L{int} contains stale value {int}")]
fn layer_stale(world: &mut FsmWorld, layer: u8, value: u32) -> Result<(), Error> {
    let state = CacheState::Stale(value);
    match layer {
        1 => world.composition.l1_state = state,
        2 => world.composition.l2_state = state,
        _ => return Err(anyhow::anyhow!("Unknown layer: L{}", layer)),
    }
    Ok(())
}

// =============================================================================
// Selective Config Steps
// =============================================================================

fn ensure_selective_config(world: &mut FsmWorld, index: usize) -> Result<(), Error> {
    if index == 0 {
        return Err(anyhow!("Config index must be 1-based"));
    }
    if index > world.selective_configs.len() {
        return Err(anyhow!(
            "Config {} not initialized. Use 'Given N selective configs' first.",
            index
        ));
    }
    Ok(())
}

#[given(expr = "{int} selective configs")]
fn selective_configs(world: &mut FsmWorld, count: usize) -> Result<(), Error> {
    world.selective_configs = (0..count)
        .map(|_| FsmConfig {
            cache_enabled: true,
            request_cacheable: true,
            response_cacheable: true,
            concurrency: world.config.concurrency,
            ttl: world.config.ttl,
            stale: world.config.stale,
        })
        .collect();
    Ok(())
}

#[given(expr = "config {int} request is cacheable")]
fn selective_request_cacheable(world: &mut FsmWorld, index: usize) -> Result<(), Error> {
    ensure_selective_config(world, index)?;
    world.selective_configs[index - 1].request_cacheable = true;
    Ok(())
}

#[given(expr = "config {int} request is non-cacheable")]
fn selective_request_non_cacheable(world: &mut FsmWorld, index: usize) -> Result<(), Error> {
    ensure_selective_config(world, index)?;
    world.selective_configs[index - 1].request_cacheable = false;
    Ok(())
}

#[given(expr = "config {int} response is cacheable")]
fn selective_response_cacheable(world: &mut FsmWorld, index: usize) -> Result<(), Error> {
    ensure_selective_config(world, index)?;
    world.selective_configs[index - 1].response_cacheable = true;
    Ok(())
}

#[given(expr = "config {int} response is non-cacheable")]
fn selective_response_non_cacheable(world: &mut FsmWorld, index: usize) -> Result<(), Error> {
    ensure_selective_config(world, index)?;
    world.selective_configs[index - 1].response_cacheable = false;
    Ok(())
}

#[given(expr = "config {int} cache policy is {string}")]
fn selective_cache_policy(world: &mut FsmWorld, index: usize, policy: String) -> Result<(), Error> {
    ensure_selective_config(world, index)?;
    match policy.as_str() {
        "Enabled" => world.selective_configs[index - 1].cache_enabled = true,
        "Disabled" => world.selective_configs[index - 1].cache_enabled = false,
        _ => return Err(anyhow!("Unknown cache policy: {}", policy)),
    }
    Ok(())
}
