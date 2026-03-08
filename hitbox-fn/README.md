# hitbox-fn

Function memoization for the Hitbox caching framework.

This crate provides tools for caching async function results using the hitbox FSM.
It wraps function arguments and return types into the hitbox pipeline, handling
cache key generation, serialization, and upstream execution.

## Overview

- **[`Cache`]** - Pre-configured cache with backend, policy, and concurrency settings
- **[`#[cached]`]** - Attribute macro that transforms async functions into cacheable functions
- **[`KeyExtract`]** - Trait for types to describe their cache key contribution
- **[`Arg`] / [`Skipped`]** - Wrappers to include or exclude function parameters from cache keys
- **[`FnExtractor`]** - Bridges `KeyExtract` to hitbox's `Extractor` trait
- **[`FnUpstream`]** - Adapts async functions to hitbox's `Upstream` trait

## Quick Start

```rust,ignore
use hitbox_fn::prelude::*;
use hitbox_moka::MokaBackend;
use std::time::Duration;

#[derive(KeyExtract)]
struct UserId(u64);

#[cached]
async fn fetch_user(id: UserId) -> Result<User, Error> {
    // expensive operation
}

// Build a reusable cache
let cache = Cache::builder()
    .backend(MokaBackend::builder().max_entries(1000).build())
    .policy(PolicyConfig::builder().ttl(Duration::from_secs(60)).build())
    .build();

// Call through cache
let user = fetch_user(UserId(42))
    .cache(&cache)
    .await?;

// With cache context (hit/miss/stale info)
let (user, ctx) = fetch_user(UserId(42))
    .cache(&cache)
    .with_context()
    .await;

// Or call directly without caching (passthrough)
let user = fetch_user(UserId(42)).await?;
```

## Feature Flags

- `derive` - Enable derive macros (`#[cached]`, `#[derive(KeyExtract)]`, etc.) via `hitbox-derive`
- `rkyv_format` - Enable rkyv zero-copy serialization support
