use actix_cache::settings::{CacheSettings, Status};
use actix_cache::dev::MockAdapter;
use actix_cache::states::initial::InitialState;
use actix_cache::states::upstream_polled::UpstreamPolled;
use actix_cache::settings::InitialCacheSettings;
use actix_cache::CacheError;
use actix_cache::states::cache_polled::CachePolled;

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
    let result = match initial_state.poll_cache().await {
        CachePolled::Successful(state) => Ok(
            state.finish().result()
        ),
        CachePolled::Miss(state) => match state.poll_upstream().await {
            UpstreamPolled::Successful(state) => Ok(
                state.update_cache().await.finish().result()
            ),
            UpstreamPolled::Error(error) => Err(error.finish().result()),
        },
        CachePolled::Error(state) => match state.poll_upstream().await {
            UpstreamPolled::Successful(state) => Ok(
                state.update_cache().await.finish().result()
            ),
            UpstreamPolled::Error(error) => Err(error.finish().result()),
        },
    };
    assert_eq!(result.unwrap(), 42);
}
