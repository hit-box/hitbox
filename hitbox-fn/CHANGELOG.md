# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- Compile-fail snapshot tests run only on the MSRV, so newer compiler diagnostics don't break them ([#PR](https://github.com/hit-box/hitbox/pull/PR))

## [0.2.1] - 2026-02-17

### Added
- `#[cached]` functions can now be called with `.await` directly without cache configuration — acts as a transparent passthrough to the underlying function ([#252](https://github.com/hit-box/hitbox/pull/252))

## [0.2.0] - 2026-02-09
### Added
- Initial release
