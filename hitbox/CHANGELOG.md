# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
