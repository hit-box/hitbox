# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `extractor()` entry point and Config-based API for all extractors
- `Into<Operation>` shorthands for predicates (e.g., `.method(Method::GET)`, `.status(StatusCode::OK)`)
- `Into<Config>` shorthands for extractors (e.g., `.path("/users/{id}")`, `.query("page")`)

### Changed
- Extractor chain methods now take Config types instead of raw values
- `PathConfig::new()` renamed to `PathConfig::pattern()`

## [0.2.1] - 2026-02-09

### Fixed
- Fall back to body hash on JSON parse failure in jq extractor to avoid cache key collisions ([#211](https://github.com/hit-box/hitbox/issues/211))
- Return errors instead of silent defaults in rkyv deserialization for HTTP version, status code, and headers ([#213](https://github.com/hit-box/hitbox/issues/213))

## [0.2.0] - 2026-01-27
### Added
- Initial release
