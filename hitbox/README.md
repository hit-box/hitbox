# hitbox

[![Build status](https://github.com/hit-box/hitbox/actions/workflows/CI.yml/badge.svg)](https://github.com/hit-box/hitbox/actions?query=workflow)
[![Coverage Status](https://codecov.io/gh/hit-box/hitbox/branch/master/graph/badge.svg?token=tgAm8OBLkY)](https://codecov.io/gh/hit-box/hitbox)

Hitbox is an asynchronous caching framework supporting multiple backends and suitable
for distributed and for single-machine applications.

## Framework integrations
- [x] [Actix](https://github.com/hit-box/hitbox/tree/master/hitbox-actix)
- [ ] Actix-Web

 ## Features
 - [x] Automatic cache key generation.
 - [x] Multiple cache backend implementations:
 - [x] Stale cache mechanics.
 - [ ] Cache locks for [dogpile effect] preventions.
 - [ ] Distributed cache locks.
 - [ ] Detailed metrics out of the box.

## Backend implementations
- [x] [Redis](https://github.com/hit-box/hitbox/tree/master/hitbox-backend)
- [ ] In-memory backend

 ## Feature flags
 * derive - Support for [Cacheable] trait derive macros.
 * metrics - Support for metrics.

 ## Restrictions
 Default cache key implementation based on serde_qs crate
 and have some [restrictions](https://docs.rs/serde_qs/latest/serde_qs/#supported-types).

## Documentation
* [API Documentation](https://docs.rs/hitbox/)
* [Examples](https://github.com/hit-box/hitbox/tree/master/examples/examples)

## Example

Dependencies:

```toml
[dependencies]
hitbox = "0.1"
```

Code:

> **_NOTE:_** Default cache key implementation based on serde_qs crate
> and have some [restrictions](https://docs.rs/serde_qs/latest/serde_qs/#supported-types).

First, you should derive [Cacheable] trait for your struct or enum:

```rust
use hitbox::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Cacheable, Serialize)] // With features=["derive"]
struct Ping {
    id: i32,
}
```
Or implement that trait manually:

```rust
use hitbox::{Cacheable, CacheError};
struct Ping { id: i32 }
impl Cacheable for Ping {
    fn cache_key(&self) -> Result<String, CacheError> {
        Ok(format!("{}::{}", self.cache_key_prefix(), self.id))
    }

    fn cache_key_prefix(&self) -> String { "Ping".to_owned() }
}
```

[Cacheable]: https://docs.rs/hitbox/latest/hitbox/cache/trait.Cacheable.html
[CacheableResponse]: https://docs.rs/hitbox/latest/hitbox/response/trait.CacheableResponse.html
[Backend]: https://docs.rs/hitbox/latest/hitbox/dev/trait.Backend.html
[RedisBackend]: https://docs.rs/hitbox-redis/latest/hitbox_redis/
[hitbox-actix]: https://docs.rs/hitbox-actix/latest/hitbox_actix/
[dogpile effect]: https://www.sobstel.org/blog/preventing-dogpile-effect/