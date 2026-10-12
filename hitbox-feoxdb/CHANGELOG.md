# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Records store their creation time and tags for tag invalidation. Records written by earlier versions stay readable and are treated as created at the Unix epoch, and earlier versions can still read new records ([#280](https://github.com/hit-box/hitbox/pull/280))

## [0.2.1] - 2026-02-09

### Fixed
- Eliminate TOCTOU race in `remove` by using a single atomic `delete` call ([#216](https://github.com/hit-box/hitbox/issues/216))

## [0.2.0] - 2026-01-27
### Added
- Initial release
