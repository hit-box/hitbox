use hitbox::dev::MockAdapter;
use hitbox::settings::InitialCacheSettings;
use hitbox::settings::{CacheSettings, Status};
use hitbox::states::cache_polled::CachePolled;
use hitbox::states::initial::InitialState;
use hitbox::states::upstream_polled::UpstreamPolled;
use hitbox::transition_groups::stale;
use hitbox::CacheError;

#[actix_rt::test]
async fn test_cache_stale() {
    let settings = CacheSettings {
        cache: Status::Enabled,
        stale: Status::Enabled,
        lock: Status::Disabled,
    };
    let adapter = MockAdapter::build()
        .with_upstream_value(42)
        .with_cache_stale()
        .with_cache_actual()
        .finish();
    let initial_state = InitialCacheSettings::from(settings);
    let initial_state = InitialState {
        adapter,
        settings: initial_state,
    };
    let finish = stale::transition(initial_state).await;
    assert_eq!(finish.result().unwrap(), 42);
}
