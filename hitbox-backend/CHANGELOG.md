# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- `Backend::invalidate()` and `Backend::invalidated()` with default implementations that store tag invalidation timestamps as regular entries under `Backend::tag_key_prefix()` (`__hitbox_tag` by default), so every backend supports tag invalidation ([#280](https://github.com/hit-box/hitbox/pull/280))
- `CompositionBackend` records invalidations in both layers and merges lookups so the latest timestamp per tag wins ([#280](https://github.com/hit-box/hitbox/pull/280))
- `serialize_tags` / `deserialize_tags` helpers for backends that store tags alongside entries ([#280](https://github.com/hit-box/hitbox/pull/280))

### Changed
- **Breaking:** `Compressor::compress` and `Compressor::decompress` take and return `Raw` (`Bytes`) instead of `&[u8]` → `Vec<u8>`, so `PassthroughCompressor` returns the value without copying it ([#300](https://github.com/hit-box/hitbox/pull/300))
- `CompositionBackend` behind `dyn Backend` or nested in another composition stores a payload shared by both layers once in its envelope, halving the copies per write when L1 and L2 use the same format without compression ([#303](https://github.com/hit-box/hitbox/pull/303))

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

