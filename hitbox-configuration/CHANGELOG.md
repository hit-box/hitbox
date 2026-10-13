# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `tags:` endpoint section with `request` and `response` lists: request tags accept `Static` and every extractor variant (`Path`, `Method`, `Query`, `Header`, `Body`, `Version`), response tags accept `Static` and `Header` ([#280](https://github.com/hit-box/hitbox/pull/280))
- `Static` tag form: a string or a list emits literal tags, a mapping emits `key=value` tags ([#280](https://github.com/hit-box/hitbox/pull/280))
- `tag_invalidation: Check | Skip` under `policy: Enabled` ([#280](https://github.com/hit-box/hitbox/pull/280))
- `HeaderOperation::into_header_config()` ([#280](https://github.com/hit-box/hitbox/pull/280))

### Added
- Initial release
- `Endpoint` now implements `CacheConfigs` for use with `SelectiveCacheFuture` ([#253](https://github.com/hit-box/hitbox/pull/253))

### Changed
- `Endpoint` now stores `policy` as `Arc<PolicyConfig>` ([#253](https://github.com/hit-box/hitbox/pull/253))
- Adapted to hitbox-http 0.3 Config-based extractor/predicate API, added `Transform::Truncate` ([#202](https://github.com/hit-box/hitbox/pull/202))

### Fixed
- README doctest missing type annotations for `Endpoint::builder()` ([#253](https://github.com/hit-box/hitbox/pull/253))
