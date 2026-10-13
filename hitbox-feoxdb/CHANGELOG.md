# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **Breaking:** Require `feoxdb` 0.6; `FeOxDbError::FeOxDb` now wraps its `FeoxError`. `feoxdb` 0.6 can refuse to open a 0.5 data file with ambiguous deletion markers: delete the cache file or convert it with feoxdb's `feox-migrate` tool ([#293](https://github.com/hit-box/hitbox/pull/293))
- **Breaking:** `FeOxDbBackend::flush` returns `Result<(), FeOxDbError>` instead of discarding sync errors ([#293](https://github.com/hit-box/hitbox/pull/293))

### Fixed
- Deadlock when a write batch overflows `max_file_size`, caused by `feoxdb` 0.5.3+ re-locking its free-space map while still holding it ([#293](https://github.com/hit-box/hitbox/pull/293))

## [0.2.1] - 2026-02-09

### Fixed
- Eliminate TOCTOU race in `remove` by using a single atomic `delete` call ([#216](https://github.com/hit-box/hitbox/issues/216))

## [0.2.0] - 2026-01-27
### Added
- Initial release
