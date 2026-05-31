# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `ValueEnvelope`: a reusable, zero-copy value envelope for backends without native TTL/metadata storage (e.g. S3, FeOxDB). Packs `expire`/`stale` into a fixed little-endian header (with a version byte for forward compatibility) followed by the raw, un-re-serialized payload bytes.

## [0.2.1] - 2026-02-05

### Changed
- Race policies use `Offload::register` instead of deprecated `spawn` ([#204](https://github.com/hit-box/hitbox/pull/204))

## [0.2.0] - 2026-01-27
### Changed
- Complete rewrite with protocol-agnostic core
- Migrated from actix to tokio/tower ecosystem

## [0.1.0] - 2021-05-29
### Added
- Initial release

