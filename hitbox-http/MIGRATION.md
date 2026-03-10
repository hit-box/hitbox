# Migration Guide

## Migrating from 0.2 to 0.3

### Extractors

Entry point changed to `request::extractor()` with Config-based chaining. String arguments accepted directly.

```rust
// Before
Method::new().path("/users/{id}").query("page".to_string())
// After
request::extractor().method(MethodConfig::new()).path("/users/{id}").query("page")
```

### Predicates

Operation wrappers removed — pass values directly via `Into<Operation>`.

```rust
// Before
request::predicate().method(method::Operation::eq(Method::GET)).path(path::Operation::pattern("/users/{id}"))
// After
request::predicate().method(Method::GET).path("/users/{id}")
```

### Body Extractor

Raw `BodyExtraction` enum replaced by `BodyConfig` builder with `.hash()`, `.jq()`, `.regex()` modes.

```rust
// Before
.body(BodyExtraction::Hash)
// After
.body(BodyConfig::new().hash())
.body(BodyConfig::new().regex(r"token=(\w+)")?.key("api-token").global())
```

### Hash Transform

`Transform::Hash` now returns full 64-char SHA256 (was 16-char truncated). Chain `Truncate` for old behavior.

```rust
// Before: 16-char hash
.transform(Transform::Hash)
// After: 64-char hash, or truncate explicitly
.transform(Transform::Hash).transform(Transform::Truncate(16))
```

### Body Transforms Builder

`Transforms::builder()` replaces manual `Transforms::FullBody(vec![...])` / `Transforms::PerKey(HashMap)` construction. Typestate prevents mixing `.full()` and `.key()`.

```rust
// Before
.transforms(Transforms::FullBody(vec![Transform::Hash]))
// After
.transforms(Transforms::builder().full(Transform::Hash))
.transforms(Transforms::builder().key("token", Transform::Hash).key("name", Transform::Lowercase))
```

