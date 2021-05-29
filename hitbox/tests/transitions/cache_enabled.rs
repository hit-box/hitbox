use hitbox::dev::MockAdapter;
use hitbox::settings::{CacheSettings, Status};
use hitbox::states::initial::Initial;
use hitbox::transition_groups::only_cache;

#[actix_rt::test]
async fn test_cache_enabled_cache_miss() {
    let settings = CacheSettings {
        cache: Status::Enabled,
        stale: Status::Disabled,
        lock: Status::Disabled,
    };
    let adapter = MockAdapter::build()
        .with_upstream_value(42)
        .with_cache_miss()
        .finish();
    let initial_state = Initial::new(settings, adapter);
    let finish = only_cache::transition(initial_state).await;
    assert_eq!(finish.result().unwrap(), 42);
}

#[actix_rt::test]
async fn test_cache_enabled_cache_hit() {
    let settings = CacheSettings {
        cache: Status::Enabled,
        stale: Status::Disabled,
        lock: Status::Disabled,
    };
    let adapter = MockAdapter::build()
        .with_upstream_error()
        .with_cache_actual(42)
        .finish();
    let initial_state = Initial::new(settings, adapter);
    let finish = only_cache::transition(initial_state).await;
    assert_eq!(finish.result().unwrap(), 42);
}

#[actix_rt::test]
async fn test_cache_enabled_cache_miss_upstream_error() {
    let settings = CacheSettings {
        cache: Status::Enabled,
        stale: Status::Disabled,
        lock: Status::Disabled,
    };
    let adapter: MockAdapter<i32> = MockAdapter::build()
        .with_upstream_error()
        .with_cache_miss()
        .finish();
    let initial_state = Initial::new(settings, adapter);
    let finish = only_cache::transition(initial_state).await;
    assert!(finish.result().is_err());
}
