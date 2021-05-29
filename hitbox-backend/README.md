# hitbox-backend

[![Build status](https://github.com/hit-box/hitbox/actions/workflows/CI.yml/badge.svg)](https://github.com/hit-box/hitbox/actions?query=workflow)

[Hitbox] is an asynchronous caching framework supporting multiple backends and suitable 
for distributed and for single-machine applications.

Hitbox Backend is the core primitive for Hitbox. 
Trait [Backend] representing the functions required to interact with cache backend.
If you want to implement your own backend, you in the right place.

## Examples
* [Async backend](https://github.com/hit-box/hitbox/blob/master/examples/examples/async_backend.rs)
* [Sync backend](https://github.com/hit-box/hitbox/blob/master/examples/examples/sync_backend.rs)

[Backend]: https://docs.rs/hitbox_backend/trait.Backend.html
[Hitbox]: https://github.com/hit-box/hitbox
