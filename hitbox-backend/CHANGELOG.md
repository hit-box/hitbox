# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2] - 2026-10-30

### Security
- Require `zstd` 0.14, which fixes RUSTSEC-2026-0391, RUSTSEC-2026-0398 and RUSTSEC-2026-0403 in the `zstd` feature ([#307](https://github.com/hit-box/hitbox/pull/307))

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

