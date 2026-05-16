use crate::app::app;
use crate::core::{HitboxWorld, StepExt};
use anyhow::{Error, anyhow};
use axum_test::TestServer;
use chrono::Utc;
use cucumber::gherkin::Step;
use cucumber::when;
use hitbox::concurrency::BroadcastConcurrencyManager;
use hitbox::tag::CacheTag;
use hitbox_backend::Backend;
use hitbox_tower::Cache;
use hurl::{
    runner::{VariableSet, request::eval_request},
    util::path::ContextDir,
};
use hurl_core::{error::DisplaySourceError, parser::parse_hurl_file, text::Format};
use std::str::FromStr;
use std::time::Duration;

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
    let parsed_request = &hurl_file
        .entries
        .first()
        .ok_or_else(|| anyhow!("request not found"))?
        .request;

    let request = eval_request(parsed_request, &variables, &ContextDir::default())
        .map_err(|err| anyhow!("hurl request error {:?}", err))?;

    world.execute_request(&request).await?;
    Ok(())
}

#[when(expr = "tag {string} is invalidated")]
async fn invalidate_tag(world: &mut HitboxWorld, tag: String) -> Result<(), Error> {
    let cache_tag = CacheTag::new(tag);
    world
        .backend
        .invalidate(&cache_tag)
        .await
        .map_err(|err| anyhow!("backend invalidate failed: {err:?}"))?;
    Ok(())
}

#[when(expr = "sleep {int}")]
async fn sleep(_world: &mut HitboxWorld, secs: u16) -> Result<(), Error> {
    tokio::time::sleep(tokio::time::Duration::from_secs(secs.into())).await;
    Ok(())
}

#[when(expr = "sleep {int}ms")]
async fn sleep_ms(_world: &mut HitboxWorld, millis: u64) -> Result<(), Error> {
    tokio::time::sleep(tokio::time::Duration::from_millis(millis)).await;
    Ok(())
}

#[when(expr = "wait for background tasks")]
async fn wait_for_background_tasks(world: &mut HitboxWorld) -> Result<(), Error> {
    if let Some(manager) = &world.offload_manager {
        // Wait up to 5 seconds for all tasks to complete
        let completed = manager
            .wait_all_timeout(std::time::Duration::from_secs(5))
            .await;
        if !completed {
            return Err(anyhow!("Background tasks did not complete within timeout"));
        }
    }
    Ok(())
}

#[when(expr = "{int} concurrent requests are made with delay {int}ms")]
async fn concurrent_requests(
    world: &mut HitboxWorld,
    num_requests: usize,
    delay_ms: u64,
    step: &Step,
) -> Result<(), Error> {
    use std::sync::Arc;

    // Parse the hurl request
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
    let parsed_request = &hurl_file
        .entries
        .first()
        .ok_or_else(|| anyhow!("request not found"))?
        .request;

    let request_spec = eval_request(parsed_request, &variables, &ContextDir::default())
        .map_err(|err| anyhow!("hurl request error {:?}", err))?;

    // Build the cache layer with concurrency manager for dogpile prevention
    let concurrency_manager: BroadcastConcurrencyManager<_> = BroadcastConcurrencyManager::new();
    let config = world.config.build();

    // Create a single TestServer and wrap in Arc for sharing
    let server = if let Some(manager) = &world.offload_manager {
        let cache = Cache::builder()
            .backend(world.backend.clone())
            .config(config)
            .concurrency_manager(concurrency_manager)
            .offload(manager.clone())
            .build();
        let router = app(world.handler_state.clone()).layer(cache);
        Arc::new(TestServer::new(router)?)
    } else {
        let cache = Cache::builder()
            .backend(world.backend.clone())
            .config(config)
            .concurrency_manager(concurrency_manager)
            .build();
        let router = app(world.handler_state.clone()).layer(cache);
        Arc::new(TestServer::new(router)?)
    };

    // Clear previous responses
    world.state.responses.clear();

    // Prepare request data
    let path = request_spec.url.path().to_string();
    let method = request_spec.method.0.to_string();
    let headers: Vec<_> = request_spec
        .headers
        .iter()
        .map(|h| (h.name.clone(), h.value.clone()))
        .collect();
    let query_params: Vec<_> = request_spec
        .url
        .query_params()
        .iter()
        .map(|p| (p.name.clone(), p.value.clone()))
        .chain(
            request_spec
                .querystring
                .iter()
                .map(|p| (p.name.clone(), p.value.clone())),
        )
        .collect();

    // Spawn concurrent requests
    let handles: Vec<_> = (0..num_requests)
        .map(|i| {
            let server = Arc::clone(&server);
            let path = path.clone();
            let method = method.clone();
            let headers = headers.clone();
            let query_params = query_params.clone();

            tokio::spawn(async move {
                // Stagger requests
                if delay_ms > 0 && i > 0 {
                    tokio::time::sleep(Duration::from_millis(i as u64 * delay_ms)).await;
                }

                let mut request = server.method(http::Method::from_str(&method).unwrap(), &path);

                for (name, value) in &headers {
                    request = request.add_header(name, value);
                }

                for (name, value) in &query_params {
                    request = request.add_query_param(name, value);
                }

                request.await
            })
        })
        .collect();

    // Collect all responses
    for handle in handles {
        let response = handle.await.map_err(|e| anyhow!("Task failed: {}", e))?;
        world.state.responses.push(response);
    }

    Ok(())
}

#[when(expr = "debug cache")]
async fn debug_cache(world: &mut HitboxWorld) -> Result<(), Error> {
    use hitbox_backend::{Backend, CacheBackend};
    use hitbox_http::CacheableHttpResponse;

    eprintln!("=== DEBUG CACHE STATE ===");
    eprintln!("Cache entry count: {}", world.backend.cache_entry_count());
    eprintln!("Backend read count: {}", world.backend.read_count());
    eprintln!("Backend write count: {}", world.backend.write_count());

    for entry in world.backend.cache.iter() {
        let key = entry.key();
        let value = entry.value();
        eprintln!("  Key: {:?}", key);
        eprintln!("  Value expire: {:?}", value.expire());
        eprintln!("  Value stale: {:?}", value.stale());

        // Test Backend::read
        let backend_read = world.backend.read(key).await;
        eprintln!(
            "  Backend read: {:?}",
            backend_read.is_ok() && backend_read.as_ref().unwrap().is_some()
        );

        // Test full CacheBackend::get (with deserialization)
        let mut ctx = hitbox_core::CacheContext::default().boxed();
        let cache_get_result = world
            .backend
            .get::<CacheableHttpResponse<axum::body::Body>>(key, &mut ctx)
            .await;
        match &cache_get_result {
            Ok(Some(cached_value)) => {
                eprintln!("  CacheBackend::get: Ok(Some)");
                eprintln!("  CacheValue expire: {:?}", cached_value.expire());
                eprintln!("  CacheValue stale: {:?}", cached_value.stale());
                // Test cache_state (sync operation - just checks timestamps)
                let cache_state = cached_value.clone().cache_state();
                match cache_state {
                    hitbox::CacheState::Actual(_) => eprintln!("  cache_state: Actual (HIT)"),
                    hitbox::CacheState::Stale(_) => eprintln!("  cache_state: Stale"),
                    hitbox::CacheState::Expired(_) => eprintln!("  cache_state: Expired (MISS)"),
                }
            }
            Ok(None) => eprintln!("  CacheBackend::get: Ok(None)"),
            Err(e) => eprintln!("  CacheBackend::get: Err({:?})", e),
        }
    }
    eprintln!("Real time now: {:?}", Utc::now());
    eprintln!("=========================");
    Ok(())
}
