# hitbox

[![Build status](https://github.com/hit-box/hitbox/actions/workflows/CI.yml/badge.svg)](https://github.com/hit-box/hitbox/actions?query=workflow)
[![Coverage Status](https://codecov.io/gh/hit-box/hitbox/branch/master/graph/badge.svg?token=tgAm8OBLkY)](https://codecov.io/gh/hit-box/hitbox)

Actix cache is a proxy actor and infrastructure for asynchronous and clear cache interaction for Actix actor and Actix-web frameworks.

## Features
* Async/Sync cache backend support.
* [Dogpile] effect prevention.
* Stale cache mechanics.
* Automatic cache key generation.
* Detailed Prometheus metrics out of the box.

## Backend implementations

At this time supported or planned next cache backend implementation:
- [x] Redis backend (hitbox-redis)
- [ ] In-memory backend

## Feature flags
* redis - Enabled by default. Add support for redis backend.
* derive - Support for [Cacheable] trait derive macros.
* metrics - Support for Prometheus metrics.

## Example

Dependencies:

```toml
[dependencies]
hitbox = "0.2"
```

Code:

First of all, you should derive [Cacheable] trait for your actix Message:

> **_NOTE:_** Default cache key implementation based on serde_qs crate
> and have some [restrictions](https://docs.rs/serde_qs/latest/serde_qs/#supported-types).


```rust
use actix::prelude::*;
use hitbox::Cacheable; // With features=["derive"]
use actix_derive::Message;
use serde::{Deserialize, Serialize};
struct Pong;

#[derive(Message, Cacheable, Serialize)]
#[rtype(result = "Result<Pong, ()>")]
struct Ping {
    id: i32,
}
```
Or implement that trait manually:

```rust
use hitbox::{Cacheable, CacheError};

struct Ping { id: i32 }

impl Cacheable for Ping {
    fn cache_message_key(&self) -> Result<String, CacheError> {
        Ok(format!("{}::{}", self.cache_key_prefix(), self.id))
    }
    fn cache_key_prefix(&self) -> String { "Ping".to_owned() }
}
```
Next step is to instantiate [CacheActor] with default backend:

```rust
use actix::prelude::*;
use hitbox::{CacheError, Cache};

#[actix_rt::main]
async fn main() -> Result<(), CacheError> {
    let cache = Cache::new()
        .await?
        .start();
   Ok(())
}
```

And the last step is using cache in your code (actix-web handler for example).
This full example and other examples you can see on [github.com](https://github.com/rambler-digital-solutions/hitbox/blob/master/examples/actix_web.rs)

```rust
use actix::prelude::*;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use hitbox::{Cache, Cacheable};
use serde::Serialize;

struct FibonacciActor;

impl Actor for FibonacciActor { type Context = Context<Self>; }

#[derive(Message, Cacheable, Serialize)]
#[rtype(result = "u64")]
struct GetNumber {
    number: u8
}

impl Handler<GetNumber> for FibonacciActor {
    type Result = <GetNumber as Message>::Result;

    fn handle(&mut self, msg: GetNumber, _ctx: &mut Self::Context) -> Self::Result {
        42
    }
}

async fn index(
    fib: web::Data<Addr<FibonacciActor>>,
    cache: web::Data<Addr<Cache>>
) -> impl Responder {
    let query = GetNumber { number: 40 };
    let number = cache
        .send(query.into_cache(&fib))
        .await
        .unwrap()
        .unwrap();
    HttpResponse::Ok().body(format!("Generate Fibonacci number {}", number))
}
```


[Dogpile]: https://www.sobstel.org/blog/preventing-dogpile-effect/
[Cacheable]: https://docs.rs/hitbox/latest/hitbox/cache/trait.Cacheable.html
[CacheActor]: https://docs.rs/hitbox/latest/hitbox/actor/struct.CacheActor.html
