# actix-cache

[![Build Status](https://travis-ci.org/rambler-digital-solutions/actix-cache.svg?branch=master)](https://travis-ci.org/rambler-digital-solutions/actix-cache)
[![Coverage Status](https://coveralls.io/repos/github/rambler-digital-solutions/actix-cache/badge.svg?branch=master)](https://coveralls.io/github/rambler-digital-solutions/actix-cache?branch=master)

Actix cache is a proxy actor and infrastructure for asynchronous and clear cache interaction for Actix actor and Actix-web frameworks.

## Features
* Async/Sync cache backend support.
* [Dogpile] effect prevention.
* Stale cache mechanics.
* Automatic cache key generation.
* Detailed Prometheus metrics out of the box.

## Backend implementations

At this time supported or planned next cache backend implementation:
- [x] Redis backend (actix-cache-redis)
- [ ] In-memory backend

## Feature flags
* derive - Support for [Cacheable] trait derive macros.
* metrics - Support for Prometheus metrics.

## Example

Dependencies:

```toml
[dependencies]
actix-cache = "0.2"
```

Code:

First of all, you should derive [Cacheable] trait for your actix Message:

> **_NOTE:_** Default cache key implementation based on serde_qs crate
> and have some [restrictions](https://docs.rs/serde_qs/latest/serde_qs/#supported-types).


```rust
use actix::prelude::*;
use actix_cache::Cacheable; // With features=["derive"]
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
use actix_cache::{Cacheable, CacheError};

struct Ping { id: i32 }

impl Cacheable for Ping {
    fn cache_message_key(&self) -> Result<String, CacheError> {
        Ok(format!("{}::{}", self.cache_key_prefix(), self.id))
    }
    fn cache_key_prefix(&self) -> String { "Ping".to_owned() }
}
```
Next step is to instantiate [Cache] actor with selected backend:

```rust
use actix::prelude::*;
use actix_cache::{CacheError, Cache as CacheActor, RedisBackend};

type Cache = CacheActor<RedisBackend>;

#[actix_rt::main]
async fn main() -> Result<(), CacheError> {
    let cache = Cache::new()
        .await?
        .start();
   Ok(())
}
```

And the last step is using cache in your code (actix-web handler for example).
This full example and other examples you can see on [github.com](https://github.com/rambler-digital-solutions/actix-cache/blob/master/examples/actix_web.rs)

```rust
use actix::prelude::*;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_cache::{Cache as CacheActor, RedisBackend, Cacheable};
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

type Cache = CacheActor<RedisBackend>;

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
[Cacheable]: https://docs.rs/actix-cache/latest/actix-cache/cache/trait.Cacheable.html
[Cache]: https://docs.rs/actix-cache/latest/actix-cache/actor/struct.Cache.html
