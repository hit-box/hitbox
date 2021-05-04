use hitbox::settings::{CacheSettings, Status};
use hitbox::dev::MockAdapter;
use hitbox::states::initial::InitialState;
use hitbox::states::upstream_polled::UpstreamPolled;
use hitbox::settings::InitialCacheSettings;
use hitbox::CacheError;
use hitbox::states::cache_polled::CachePolled;
use hitbox::transition_groups::only_cache;

#[actix_rt::test]
async fn test_cache_enabled_cache_polled_successful() {
    let settings = CacheSettings {
        cache: Status::Enabled,
        stale: Status::Disabled,
        lock: Status::Disabled,
    };
    let adapter = MockAdapter::build()
        .with_upstream_value(42)
        .with_cache_actual()
        .finish();
    let initial_state = InitialCacheSettings::from(settings);
    let initial_state = InitialState { adapter, settings: initial_state };
    let finish = only_cache::transition(initial_state).await;
    assert_eq!(finish.result().unwrap(), 42);
}
