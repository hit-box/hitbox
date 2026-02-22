# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `SelectiveCacheFuture` for multi-config routing with first-match-wins strategy ([#253](https://github.com/hit-box/hitbox/pull/253))
- `SelectiveConfig` container for multiple `CacheConfig` instances ([#253](https://github.com/hit-box/hitbox/pull/253))

### Changed
- `Config` now implements `CacheConfigs` via `slice::from_ref` ([#253](https://github.com/hit-box/hitbox/pull/253))

## [0.2.0] - 2026-01-27
### Changed
- Complete rewrite with protocol-agnostic core
- Migrated from actix to tokio/tower ecosystem

## [0.1.0] - 2021-05-29
### Added
- Initial release
