# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `SelectiveCacheFuture` for multi-config routing with first-match-wins strategy ([#253](https://github.com/hit-box/hitbox/pull/253))
- FSM wiring for `CacheTiming`, `stored`, `Collapsed`, and `Forward(Bypass)` statuses ([#269](https://github.com/hit-box/hitbox/pull/269))
- `SelectiveConfig` container for multiple `CacheConfig` instances ([#253](https://github.com/hit-box/hitbox/pull/253))

### Changed
- `Config` now implements `CacheConfigs` via `slice::from_ref` ([#253](https://github.com/hit-box/hitbox/pull/253))
- `Config` now stores `policy` as `Arc<PolicyConfig>` ([#253](https://github.com/hit-box/hitbox/pull/253))

### Changed
- **Breaking:** Relaxed FSM lifetime bounds from `'static` to `'offload` for `Req`, `ReqP`, and `E` — enables caching of request types containing references ([#206](https://github.com/hit-box/hitbox/pull/206))

### Added
- Re-export `Upstream`, `Cacheable`, and `OffloadKey` from `hitbox-core` ([#206](https://github.com/hit-box/hitbox/pull/206))

## [0.2.4] - 2026-02-17

### Added
- Re-export `DisabledOffload`, `Offload`, and `serde` from `hitbox-core` — derive macros now resolve all types through `hitbox::` ([#252](https://github.com/hit-box/hitbox/pull/252))

## [0.2.3] - 2026-02-10

### Fixed
- Correct `stale` field documentation — it is time from cache write until entry becomes stale, not a window after TTL
- Remove arbitrary 5s default TTL from `EnabledCacheConfig` — default is now `None` (cache indefinitely), consistent with `PolicyConfigBuilder` and `EntityPolicyConfig`

## [0.2.2] - 2026-02-09

### Fixed
- Replace `wait_all()` busy-wait loop with `Notify`-based wakeup to avoid burning CPU ([#210](https://github.com/hit-box/hitbox/issues/210))
- Use `DashMap::entry()` to prevent TOCTOU race in offload deduplication check ([#214](https://github.com/hit-box/hitbox/issues/214))
- `OffloadManager::register` now enforces the `max_concurrent_tasks` limit ([#209](https://github.com/hit-box/hitbox/issues/209))

## [0.2.1] - 2026-02-05

### Added
- `OffloadManager::register` method with `OffloadKey` support ([#204](https://github.com/hit-box/hitbox/pull/204))

### Fixed
- SWR revalidation deduplication now works correctly via `OffloadKey::Keyed` ([#204](https://github.com/hit-box/hitbox/pull/204))

### Deprecated
- `OffloadManager::spawn` and `spawn_with_key` in favor of `register` ([#204](https://github.com/hit-box/hitbox/pull/204))

## [0.2.0] - 2026-01-27
### Changed
- Complete rewrite with protocol-agnostic core
- Migrated from actix to tokio/tower ecosystem

## [0.1.0] - 2021-05-29
### Added
- Initial release
