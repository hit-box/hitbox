#![warn(missing_docs)]

pub mod runtime;
pub mod actor;
pub mod builder;
pub mod messages;
pub mod handlers;

pub use actor::CacheActor;
pub use builder::CacheBuilder;
pub use runtime::ActixAdapter;
pub use messages::QueryCache;

use hitbox_redis::RedisBackend;

/// Default type alias with RedisBackend.
/// You can disable it or define it manually in your code.
// #[cfg(feature = "redis")]
pub type Cache = CacheActor<RedisBackend>;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
