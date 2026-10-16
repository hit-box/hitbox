# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `jq_body` benchmark comparing jq predicates and extractors with 0.2 ([#PR](https://github.com/hit-box/hitbox/pull/PR))

### Changed
- jq predicates and extractors share one parse of the body per request or response ([#PR](https://github.com/hit-box/hitbox/pull/PR))
- **Breaking:** jq runs on jaq 3; some jq-derived keys change (indexing `null`, number literal text, big integers, `NaN` bodies) ([#PR](https://github.com/hit-box/hitbox/pull/PR))
- **Breaking:** `JqExtraction` is replaced by `JqExpression` (`hitbox_http::jq`); `CacheableSubject` gains `extensions_mut`; `Operation::check` takes `&mut Extensions` ([#PR](https://github.com/hit-box/hitbox/pull/PR))

### Fixed
- jq integer overflow near `i64::MAX`/`i64::MIN` ([#PR](https://github.com/hit-box/hitbox/pull/PR))

### Added
- `ResponseHeaderTagExtractor` (`hitbox_http::tags`) tags cached entries by a response header such as `ETag`, with the same name selection, regex value extraction, and transforms as the request `Header` extractor ([#280](https://github.com/hit-box/hitbox/pull/280))
- `HeaderConfig::new()` and `HeaderConfig::select()`; the request `Header` extractor and the response tag extractor share one header-reading path ([#280](https://github.com/hit-box/hitbox/pull/280))

### Changed
- `CacheableHttpRequest` and `CacheableHttpResponse` implement the tag-aware `cache_policy` signatures ([#280](https://github.com/hit-box/hitbox/pull/280))

### Added
- Preserve HTTP/2 trailers through body buffering and cache serialization roundtrip ([#261](https://github.com/hit-box/hitbox/pull/261))

### Changed
- **Breaking:** Unified Config-based extractor/predicate API with `Into` shorthands, `Transform::Truncate`, `Transforms::builder()` with typestate, and full 64-char SHA256 for `Hash` ([#202](https://github.com/hit-box/hitbox/pull/202))
- Adapted `CacheableRequest` impl to GAT-based `CachePolicyFuture` ([#206](https://github.com/hit-box/hitbox/pull/206))
- **Breaking:** `BufferedBody::Complete` and `collect()` API changed to carry optional trailers ([#261](https://github.com/hit-box/hitbox/pull/261))

### Fixed
- Clippy `result_large_err` on `BufferedBody::collect()` and query extractor tests rejected by `http` 1.5 ([#293](https://github.com/hit-box/hitbox/pull/293))

## [0.2.1] - 2026-02-09

### Fixed
- Fall back to body hash on JSON parse failure in jq extractor to avoid cache key collisions ([#211](https://github.com/hit-box/hitbox/issues/211))
- Return errors instead of silent defaults in rkyv deserialization for HTTP version, status code, and headers ([#213](https://github.com/hit-box/hitbox/issues/213))

## [0.2.0] - 2026-01-27
### Added
- Initial release
