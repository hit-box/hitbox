# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Changed
- jq step and benches use `JqExpression`; direct jaq dependencies dropped ([#PR](https://github.com/hit-box/hitbox/pull/PR))

### Added
- Initial release
- BDD coverage for tag invalidation: request and response tags, the `Skip` policy, response `Header` tags, and invalidation during an in-flight upstream call ([#280](https://github.com/hit-box/hitbox/pull/280))
