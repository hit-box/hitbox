use hitbox::dev::MockAdapter;
use hitbox::settings::{CacheSettings, Status};
use hitbox::states::initial::Initial;
use hitbox::transition_groups::stale;

#[actix_rt::test]
async fn test_cache_stale() {
    let settings = CacheSettings {
        cache: Status::Enabled,
        stale: Status::Enabled,
        lock: Status::Disabled,
    };
    let adapter = MockAdapter::build()
        .with_upstream_value("upstream value")
        .with_cache_stale("stale cache", chrono::Utc::now())
        .finish();
    let initial_state = Initial::new(settings, adapter);
    let finish = stale::transition(initial_state).await;
    assert_eq!(finish.result().unwrap(), "upstream value");
}

#[actix_rt::test]
async fn test_upstream_error() {
    let settings = CacheSettings {
        cache: Status::Enabled,
        stale: Status::Enabled,
        lock: Status::Disabled,
    };
    let adapter = MockAdapter::build()
        .with_upstream_error()
        .with_cache_stale("stale cache", chrono::Utc::now())
        .finish();
    let initial_state = Initial::new(settings, adapter);
    let finish = stale::transition(initial_state).await;
    assert_eq!(finish.result().unwrap(), "stale cache");
}

#[actix_rt::test]
async fn test_cache_actual() {
    let settings = CacheSettings {
        cache: Status::Enabled,
        stale: Status::Enabled,
        lock: Status::Disabled,
    };
    let adapter = MockAdapter::build()
        .with_upstream_value("upstream value")
        .with_cache_actual("actual cache")
        .finish();
    let initial_state = Initial::new(settings, adapter);
    let finish = stale::transition(initial_state).await;
    assert_eq!(finish.result().unwrap(), "actual cache");
}
