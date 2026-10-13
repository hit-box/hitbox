# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Tag invalidation records live in a dedicated, unbounded store, so data eviction can never drop an invalidation ([#280](https://github.com/hit-box/hitbox/pull/280))
- `MokaBackend::tag_record_count()` and the `hitbox_moka_tag_records` gauge (`metrics` feature) ([#280](https://github.com/hit-box/hitbox/pull/280))

## [0.2.0] - 2026-01-27
### Added
- Initial release
