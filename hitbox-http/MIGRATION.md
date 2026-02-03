# Migration Guide

## Migrating from 0.2 to 0.3

This guide covers the breaking changes introduced in hitbox-http 0.3.

### Summary

Version 0.3 introduces a unified Config-based API for extractors and `Into<Operation>` shorthands for predicates, making the API more ergonomic and consistent.

### Extractors

**Before:**
```rust
use hitbox_http::extractors::{Method, path::PathExtractor, query::QueryExtractor};

let extractor = Method::new()
    .path("/users/{id}")
    .query("page".to_string())
    .header("x-api-key".to_string());
```

**After:**
```rust
use hitbox_http::extractors::{self, MethodConfig, MethodExtractor, PathExtractor};
use hitbox_http::extractors::query::QueryExtractor;
use hitbox_http::extractors::header::HeaderExtractor;

let extractor = extractors::extractor()
    .method(MethodConfig::new())
    .path("/users/{id}")
    .query("page")
    .header("x-api-key");
```

### Predicates

**Before:**
```rust
use hitbox_http::predicates::request::{self, method, path};
use hitbox_http::predicates::response::status;

request::predicate()
    .method(method::Operation::eq(Method::GET))
    .path(path::Operation::pattern("/users/{id}"))

response::predicate()
    .status(status::Operation::eq(StatusCode::OK))
```

**After:**
```rust
use hitbox_http::predicates::{request, response};

request::predicate()
    .method(Method::GET)
    .path("/users/{id}")

response::predicate()
    .status(StatusCode::OK)
```

### Body Extractor

**Before:**
```rust
.body(BodyExtraction::Hash)
.body(BodyExtraction::Regex(RegexExtraction { ... }))
```

**After:**
```rust
use hitbox_http::extractors::body::{BodyConfig, BodyExtractor};

.body(BodyConfig::new().hash())
.body(BodyConfig::new().regex(r"token=(\w+)")?.key("api-token").global())
.body(BodyConfig::new().jq(".data.id")?)
```

### Quick Reference

| Old | New |
|-----|-----|
| `Method::new()` | `extractors::extractor().method(MethodConfig::new())` |
| `.path(String)` | `.path(&str)` |
| `.query(String)` | `.query(&str)` |
| `.header(String)` | `.header(&str)` |
| `.body(BodyExtraction::Hash)` | `.body(BodyConfig::new().hash())` |
| `method::Operation::eq(M)` | `M` (direct) |
| `path::Operation::pattern(p)` | `p` (direct) |
| `status::Operation::eq(s)` | `s` (direct) |
| `PathConfig::new(p)` | `PathConfig::pattern(p)` |
