# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Changed
- Body `jq` extractors use `JqExpression` on jaq 3 ([#294](https://github.com/hit-box/hitbox/pull/294))
- **Breaking:** YAML is parsed with serde-saphyr 2.1, so `ConfigError::Yaml` wraps its `Error`, and null for a list or mapping, wrong-case `null`/`true`/`false` spellings and non-finite untyped floats are now errors; it no longer pulls a `smallvec` pre-release that needs Rust 1.100 ([#316](https://github.com/hit-box/hitbox/pull/316))
- **Breaking:** `Method` and `Version` extractors are unit structs, so `- Method:` keeps parsing under serde-saphyr 2.1 ([#316](https://github.com/hit-box/hitbox/pull/316))
- Drop the unused `bytesize` dependency ([#315](https://github.com/hit-box/hitbox/pull/315))

### Added
- `Postcard` value format ([#311](https://github.com/hit-box/hitbox/pull/311))
- `tags:` endpoint section with `request` and `response` lists: request tags accept `Static` and every extractor variant (`Path`, `Method`, `Query`, `Header`, `Body`, `Version`), response tags accept `Static` and `Header` ([#280](https://github.com/hit-box/hitbox/pull/280))
- `Static` tag form: a string or a list emits literal tags, a mapping emits `key=value` tags ([#280](https://github.com/hit-box/hitbox/pull/280))
- `tag_invalidation: Check | Skip` under `policy: Enabled` ([#280](https://github.com/hit-box/hitbox/pull/280))
- `HeaderOperation::into_header_config()` ([#280](https://github.com/hit-box/hitbox/pull/280))

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
