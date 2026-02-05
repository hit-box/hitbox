# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2026-02-05

### Added
- `OffloadKey` enum with `Keyed`, `Explicit`, and `Auto` variants ([#204](https://github.com/hit-box/hitbox/pull/204))
- `Offload::register` method as primary API for background tasks ([#204](https://github.com/hit-box/hitbox/pull/204))

### Deprecated
- `Offload::spawn` in favor of `register` ([#204](https://github.com/hit-box/hitbox/pull/204))

## [0.2.0] - 2026-01-27
### Added
- Initial release
