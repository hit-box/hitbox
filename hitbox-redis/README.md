# hitbox-backend

Hitbox [Backend] implementation for Redis.

This crate uses [redis-rs] as base library for asynchronous interaction with redis nodes.
It uses one [MultiplexedConnection] for better connection utilisation.

[MultiplexedConnection]: redis::aio::MultiplexedConnection
[Backend]: hitbox_backend::Backend
[redis-rs]: redis-rs::aio
