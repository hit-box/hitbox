# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Entries store their creation time and tags in extra hash fields (`c`, `t`) for tag invalidation. Entries written by earlier versions stay readable and are treated as created at the Unix epoch; earlier versions ignore the new fields ([#280](https://github.com/hit-box/hitbox/pull/280))
- Tag invalidation records are stored without a TTL: use a `volatile-*` `maxmemory-policy` so Redis never evicts them, since `allkeys-*` policies can silently drop an invalidation ([#280](https://github.com/hit-box/hitbox/pull/280))

## [0.2.1] - 2026-02-09

### Fixed
- Use `PEXPIRE` (milliseconds) for writes to match `PTTL` read precision, preventing TTL truncation ([#212](https://github.com/hit-box/hitbox/issues/212))

## [0.2.0] - 2026-01-27
### Changed
- Complete rewrite with protocol-agnostic core
- Migrated from actix to tokio/tower ecosystem

## [0.1.0] - 2021-05-29
### Added
- Initial release

