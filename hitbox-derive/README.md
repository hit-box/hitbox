# hitbox-derive

Derive macros for the Hitbox caching framework.

This crate provides procedural macros to reduce boilerplate when using `hitbox-fn`.

## Macros

- **`#[cached]`** - Attribute macro that transforms an async function into a cacheable function
  with a builder-style API for configuring backend, policy, and context.
  Can also be called directly with `.await` for a transparent passthrough (no caching).

- **`#[derive(KeyExtract)]`** - Derive the `KeyExtract` trait for structs to control
  how they contribute to cache keys. Supports `#[key_extract(skip)]` and
  `#[key_extract(name = "...")]` field attributes.

- **`#[derive(CacheableResponse)]`** - Derive the `CacheableResponse` trait for return types.
  Supports `#[cacheable_response(skip)]` to exclude fields from caching (reconstructed via `Default`).

- **`#[derive(CacheableRequest)]`** - Derive the `CacheableRequest` trait with standard
  cache policy logic.

## Usage

This crate is typically used through `hitbox-fn` with the `derive` feature enabled:

```toml
[dependencies]
hitbox-fn = { version = "0.2", features = ["derive"] }
```

```rust,ignore
use hitbox_fn::prelude::*;

#[derive(KeyExtract)]
struct UserId(u64);

#[derive(Clone, Serialize, Deserialize, CacheableResponse)]
struct User {
    id: u64,
    name: String,
    #[cacheable_response(skip)]
    session_token: Option<String>,
}

#[cached]
async fn fetch_user(id: UserId) -> Result<User, Error> {
    // expensive operation
}
```
