use hitbox::dev::MockAdapter;
use hitbox::settings::{CacheSettings, Status};
use hitbox::states::initial::Initial;
use hitbox::transition_groups::upstream;

#[actix::test]
async fn test_cache_disabled_upstream_polled() {
    let settings = CacheSettings {
        cache: Status::Disabled,
        stale: Status::Disabled,
        lock: Status::Disabled,
    };
    let adapter = MockAdapter::build().with_upstream_value(42).finish();
    let initial_state = Initial::new(settings, adapter);
    let finish = upstream::transition(initial_state).await;
    assert_eq!(finish.result().unwrap(), 42);
}

#[actix::test]
async fn test_cache_disabled_upstream_error() {
    let settings = CacheSettings {
        cache: Status::Disabled,
        stale: Status::Disabled,
        lock: Status::Disabled,
    };
    let adapter: MockAdapter<i32> = MockAdapter::build().with_upstream_error().finish();
    let initial_state = Initial::new(settings, adapter);
    let finish = upstream::transition(initial_state).await;
    assert!(finish.result().is_err());
}
