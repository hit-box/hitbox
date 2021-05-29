# hitbox-redis

Hitbox [Backend] implementation for Redis.

This crate uses [redis-rs] as base library for asynchronous interaction with redis nodes.
It uses one [MultiplexedConnection] for better connection utilisation.

## Example backend usage with hitbox_actix

```rust
use actix::prelude::*;
use hitbox_actix::prelude::*;

#[actix::main]
async fn main() -> Result<(), CacheError> {
    let backend = RedisBackend::new()
    	.await?
	.start();

    let cache = Cache::builder()
        .finish(backend)
        .start();
    Ok(())
}
```

[MultiplexedConnection]: redis::aio::MultiplexedConnection
[Backend]: hitbox_backend::Backend
[redis-rs]: redis-rs::aio
