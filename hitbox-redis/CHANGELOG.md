# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Entries store their creation time and tags in extra hash fields (`c`, `t`) for tag invalidation ([#280](https://github.com/hit-box/hitbox/pull/280))
- Tag invalidation records are stored without a TTL: use a `volatile-*` `maxmemory-policy` so Redis never evicts them, since `allkeys-*` policies can silently drop an invalidation ([#280](https://github.com/hit-box/hitbox/pull/280))

### Changed
- **Breaking:** keys start with hitbox-backend's `WIRE_VERSION`, so 0.3 never reads entries written by 0.2. 0.2 keys carry no version and in rare cases equal a 0.3 key: flush the database hitbox uses when upgrading from 0.2 ([#318](https://github.com/hit-box/hitbox/pull/318))
- Bump minimum `redis` dependency to 1.2 and migrate from deprecated `ClusterClientBuilder::read_from_replicas()` to `read_routing_strategy(RandomReplicaStrategy)` ([#277](https://github.com/hit-box/hitbox/pull/277))
- **Breaking:** values default to `PostcardFormat` instead of `BincodeFormat`, and tags are stored as postcard. Postcard isn't self-describing: types that use `#[serde(untagged)]`, `#[serde(flatten)]`, `#[serde(skip_serializing_if)]` or `serde_json::Value` need `JsonFormat`. Flush the database when you change the value format or compressor ([#319](https://github.com/hit-box/hitbox/pull/319))

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

