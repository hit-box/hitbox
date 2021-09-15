# hitbox-axum

*Work In Progress!*

 Hitbox-Axum is an asynchronous caching framework for [Axum] framework.
 It's designed for distributed and for single-machine applications.

 ## Features
 - [x] Automatic cache key generation.
 - [x] Multiple cache backend implementations.
 - [x] Stale cache mechanics.
 - [ ] Cache locks for [dogpile effect] preventions.
 - [ ] Distributed cache locks.
 - [ ] Detailed metrics out of the box.

## Backend implementations
- [x] [Redis](https://github.com/hit-box/hitbox/tree/master/hitbox-redis)
- [ ] In-memory backend

 ## Feature flags
 * derive - Support for [Cacheable] trait derive macros.
 * redis - Support for default redis backend.

 ## Restrictions
 Default cache key implementation based on serde_qs crate
 and have some [restrictions](https://docs.rs/serde_qs/latest/serde_qs/#supported-types).

[Cacheable]: https://docs.rs/hitbox/latest/hitbox/cache/trait.Cacheable.html
[CacheableResponse]: https://docs.rs/hitbox/latest/hitbox/response/trait.CacheableResponse.html
[Backend]: https://docs.rs/hitbox-backend/latest/hitbox_backend/trait.Backend.html
[RedisBackend]: https://docs.rs/hitbox-redis/latest/hitbox_redis/struct.RedisBackend.html
[dogpile effect]: https://www.sobstel.org/blog/preventing-dogpile-effect/

[Axum]: https://github.com/tokio-rs/axum
