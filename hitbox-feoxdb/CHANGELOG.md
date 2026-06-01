# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0]

### Changed
- **Breaking:** On-disk value format changed. Cache entries now use the unified `ValueEnvelope` from `hitbox-backend` instead of the previous `SerializableCacheValue` (double-bincode) wrapper. Existing FeOxDB cache files written by 0.2.x are invalidated on upgrade; undecodable entries are treated as a cache miss (logged at `warn`), so no manual migration is required.
- Sub-second TTL precision is now preserved. The previous path truncated expiration to whole seconds; the envelope stores full nanosecond precision.

### Fixed
- Removed the redundant second serialization pass on writes (the already-serialized payload is no longer re-encoded through bincode).

## [0.2.1] - 2026-02-09

### Fixed
- Eliminate TOCTOU race in `remove` by using a single atomic `delete` call ([#216](https://github.com/hit-box/hitbox/issues/216))

## [0.2.0] - 2026-01-27
### Added
- Initial release
