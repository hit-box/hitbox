# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Changed
- jq step and benches use `JqExpression`; direct jaq dependencies dropped ([#294](https://github.com/hit-box/hitbox/pull/294))
- `backend_format` benchmark: a postcard row, groups with a valid JSON body (1 KB to 1 MB, 6 headers) and with 20 headers, and each backend is dropped after its row ([#PR](https://github.com/hit-box/hitbox/pull/PR))

### Added
- Initial release
- BDD coverage for tag invalidation: request and response tags, the `Skip` policy, response `Header` tags, and invalidation during an in-flight upstream call ([#280](https://github.com/hit-box/hitbox/pull/280))
