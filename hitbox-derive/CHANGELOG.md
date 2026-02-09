# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `#[cached]` proc macro for transparent async function memoization ([#203](https://github.com/hit-box/hitbox/pull/203))
- `#[derive(KeyExtract)]` derive macro for automatic cache key extraction from structs ([#203](https://github.com/hit-box/hitbox/pull/203))
- `#[derive(CacheableResponse)]` derive macro for cache-aware response types with skippable fields ([#203](https://github.com/hit-box/hitbox/pull/203))

## [0.1.1] - 2021-05-30

## [0.1.0] - 2021-05-29
### Added
- Initial release
