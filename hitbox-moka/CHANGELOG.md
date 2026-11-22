# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Tag invalidation records live in a dedicated, unbounded store, so data eviction can never drop an invalidation ([#280](https://github.com/hit-box/hitbox/pull/280))
- `MokaBackend::tag_record_count()` and the `hitbox_moka_tag_records` gauge (`metrics` feature) ([#280](https://github.com/hit-box/hitbox/pull/280))

### Changed
- **Breaking:** values default to `PostcardFormat` instead of `JsonFormat`: JSON reads were 33–1100× slower than bincode on real HTTP responses, and stored bodies as arrays of numbers. Postcard isn't self-describing: types that use `#[serde(untagged)]`, `#[serde(flatten)]`, `#[serde(skip_serializing_if)]` or `serde_json::Value` need `JsonFormat` ([#PR](https://github.com/hit-box/hitbox/pull/PR))

## [0.2.0] - 2026-01-27
### Added
- Initial release
