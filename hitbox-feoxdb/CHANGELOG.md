# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2026-02-09

### Fixed
- Eliminate TOCTOU race in `remove` by using a single atomic `delete` call ([#216](https://github.com/hit-box/hitbox/issues/216))

## [0.2.0] - 2026-01-27
### Added
- Initial release
