# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `CacheConfig` and `CacheConfigs` traits for cache configuration abstraction ([#253](https://github.com/hit-box/hitbox/pull/253))
- `ForwardReason`, `CacheTiming`, and protocol extensions in `CacheContext` for richer cache operation metadata ([#269](https://github.com/hit-box/hitbox/pull/269))

### Changed
- **Breaking:** `CacheStatus` extended with `Collapsed` and `Forward(ForwardReason)` variants ([#269](https://github.com/hit-box/hitbox/pull/269))
- **Breaking:** `CacheStatusExt::cache_status()` takes `&CacheContext` instead of `CacheStatus` ([#269](https://github.com/hit-box/hitbox/pull/269))
- **Breaking:** `CacheValue::new()` and `CacheMeta::new()` take additional `created_at` argument for tracking entry creation time ([#269](https://github.com/hit-box/hitbox/pull/269))

### Changed
- `PolicyConfig` and related policy types moved from `hitbox` crate ([#253](https://github.com/hit-box/hitbox/pull/253))
- **Breaking:** `CacheConfig::policy()` now returns `Arc<PolicyConfig>` instead of `&PolicyConfig` for consistency with other trait methods ([#253](https://github.com/hit-box/hitbox/pull/253))

### Changed
- **Breaking:** `Upstream::call` now takes `self` by value instead of `&mut self` — the FSM calls upstream exactly once, so consuming is semantically correct and simplifies lifetime handling ([#206](https://github.com/hit-box/hitbox/pull/206))
- **Breaking:** `CacheableRequest::cache_policy` now uses a GAT (`CachePolicyFuture<'a, P, E>`) instead of RPITIT, allowing request types with non-`'static` references (e.g. `&'a str`) ([#206](https://github.com/hit-box/hitbox/pull/206))

## [0.2.2] - 2026-02-09

### Added
- `KeyPart::with_key()` method to replace key while keeping the value ([#203](https://github.com/hit-box/hitbox/pull/203))
- `KeyPart::prefixed()` method to add dot-separated prefix to key ([#203](https://github.com/hit-box/hitbox/pull/203))
- `CacheValue::from_config()` constructor from `EntityPolicyConfig` ([#203](https://github.com/hit-box/hitbox/pull/203))
- `CacheableResponse` implementations for scalar types (integers, `bool`, `char`, `String`) and `Vec<T>` ([#203](https://github.com/hit-box/hitbox/pull/203))

## [0.2.1] - 2026-02-05

### Added
- `OffloadKey` enum with `Keyed`, `Explicit`, and `Auto` variants ([#204](https://github.com/hit-box/hitbox/pull/204))
- `Offload::register` method as primary API for background tasks ([#204](https://github.com/hit-box/hitbox/pull/204))

### Deprecated
- `Offload::spawn` in favor of `register` ([#204](https://github.com/hit-box/hitbox/pull/204))

## [0.2.0] - 2026-01-27
### Added
- Initial release
