# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Initial release

### Fixed
- Remove redundant double `Arc::clone` in `Endpoint::clone` for the `extractors` field ([#214](https://github.com/hit-box/hitbox/issues/214))
