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

## [0.2.0] - 2026-01-27
### Added
- Initial release
