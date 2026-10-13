# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Tag-based cache invalidation primitives: `CacheTag`, `CacheTags`, the `TagExtractor` trait, `ChainTagExtractor`, `NeutralTagExtractor`, and `TagAdapter` / `ExtractorExt::into_tag_extractor()` to reuse any key `Extractor` as a tag extractor ([#280](https://github.com/hit-box/hitbox/pull/280))
- `StaticExtractor` for fixed key parts or tags ([#280](https://github.com/hit-box/hitbox/pull/280))
- `TagInvalidation` on `EnabledCacheConfig` (`Check` by default, or `Skip` to disable tag extraction and the read-time check), with `PolicyConfigBuilder::tag_invalidation()` and `PolicyConfig::tag_invalidation_enabled()` ([#280](https://github.com/hit-box/hitbox/pull/280))
- `CacheValue` carries its creation time and optional tags: `with_created()`, `created()`, `with_creation_time()`, `with_tags()`, `with_optional_tags()`, `tags()` ([#280](https://github.com/hit-box/hitbox/pull/280))

### Changed
- **Breaking:** `CacheableRequest::cache_policy` and `CacheableResponse::cache_policy` take an `Option` tag extractor and return the extracted tags alongside the policy (`None` skips extraction); the `CachePolicyFuture` GAT gains a tag-extractor parameter ([#280](https://github.com/hit-box/hitbox/pull/280))
- **Breaking:** `CacheConfig` gains `RequestTagExtractor` / `ResponseTagExtractor` associated types and `request_tag_extractors()` / `response_tag_extractors()` ([#280](https://github.com/hit-box/hitbox/pull/280))

### Added
- `CacheConfig` and `CacheConfigs` traits for cache configuration abstraction ([#253](https://github.com/hit-box/hitbox/pull/253))

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
