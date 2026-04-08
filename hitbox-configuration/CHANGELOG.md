# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Initial release
- `Endpoint` now implements `CacheConfigs` for use with `SelectiveCacheFuture` ([#253](https://github.com/hit-box/hitbox/pull/253))
- `ConfigEndpoints` wrapper type for parsing a list of endpoints from YAML, and optional `name` field on `ConfigEndpoint` / `Endpoint` / `EndpointBuilder` for identifying endpoints in multi-endpoint configurations ([#276](https://github.com/hit-box/hitbox/pull/276))
- `ConfigError::EndpointAt { index, name, source }` variant carrying positional context when `ConfigEndpoints::into_endpoints` fails on a specific entry ([#276](https://github.com/hit-box/hitbox/pull/276))

### Changed
- `Endpoint` now stores `policy` as `Arc<PolicyConfig>` ([#253](https://github.com/hit-box/hitbox/pull/253))
- Adapted to hitbox-http 0.3 Config-based extractor/predicate API, added `Transform::Truncate` ([#202](https://github.com/hit-box/hitbox/pull/202))

### Fixed
- README doctest missing type annotations for `Endpoint::builder()` ([#253](https://github.com/hit-box/hitbox/pull/253))
