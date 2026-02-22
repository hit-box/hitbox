# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Re-export `CacheConfigs` and `SelectiveConfig` ([#253](https://github.com/hit-box/hitbox/pull/253))

### Changed
- `CacheService` now requires `C: CacheConfigs` instead of `C: CacheConfig`, routing all requests through `SelectiveCacheFuture` ([#253](https://github.com/hit-box/hitbox/pull/253))

## [0.2.0] - 2026-01-27
### Added
- Initial release
