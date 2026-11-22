# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Records store their creation time and tags for tag invalidation ([#280](https://github.com/hit-box/hitbox/pull/280))

### Changed
- **Breaking:** Require `feoxdb` 0.6; `FeOxDbError::FeOxDb` now wraps its `FeoxError`. `feoxdb` 0.6 can refuse to open a 0.5 data file with ambiguous deletion markers: delete the cache file or convert it with feoxdb's `feox-migrate` tool ([#293](https://github.com/hit-box/hitbox/pull/293))
- **Breaking:** `FeOxDbBackend::flush` returns `Result<(), FeOxDbError>` instead of discarding sync errors ([#293](https://github.com/hit-box/hitbox/pull/293))
- **Breaking:** keys are encoded with the backend's `key_format` (Bitcode by default), so they carry `WIRE_VERSION`; before, `key_format` was ignored and keys were bincode. Delete the database file when upgrading from 0.2: its keys carry no version and in rare cases equal a 0.3 key ([#318](https://github.com/hit-box/hitbox/pull/318))
- **Breaking:** values default to `PostcardFormat` instead of `JsonFormat`. Postcard isn't self-describing: types that use `#[serde(untagged)]`, `#[serde(flatten)]`, `#[serde(skip_serializing_if)]` or `serde_json::Value` need `JsonFormat` ([#319](https://github.com/hit-box/hitbox/pull/319))
- **Breaking:** records are postcard instead of bincode, and records written before tag invalidation are no longer read; delete the database file when upgrading ([#319](https://github.com/hit-box/hitbox/pull/319))
- **Breaking:** `FeOxDbError` drops the `Serialization` and `Deserialization` variants, which nothing returned and which exposed bincode error types; hitbox-feoxdb no longer depends on bincode ([#319](https://github.com/hit-box/hitbox/pull/319))

### Fixed
- Deadlock when a write batch overflows `max_file_size`, caused by `feoxdb` 0.5.3+ re-locking its free-space map while still holding it ([#293](https://github.com/hit-box/hitbox/pull/293))

## [0.2.2] - 2026-10-17

### Fixed
- Deadlock in `feoxdb` 0.5.3 and 0.5.4 when the data file is full or fragmented: require `feoxdb` below 0.5.3 ([#295](https://github.com/hit-box/hitbox/issues/295), [#296](https://github.com/hit-box/hitbox/pull/296))

## [0.2.1] - 2026-02-09

### Fixed
- Eliminate TOCTOU race in `remove` by using a single atomic `delete` call ([#216](https://github.com/hit-box/hitbox/issues/216))

## [0.2.0] - 2026-01-27
### Added
- Initial release
