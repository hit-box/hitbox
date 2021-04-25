use actix_cache::settings::{CacheSettings, Status, InitialCacheSettings};
use actix_cache::adapted::runtime_adapter::RuntimeAdapter;
use actix_cache::adapted::AdapterResult;
use actix_cache::adapted::actix_runtime_adapter::CacheState;
use actix_cache::states::initial::InitialState;
use actix_cache::states::upstream_polled::UpstreamPolled;

pub struct MockAdapter;

impl RuntimeAdapter for MockAdapter {
    type UpstreamResult = i32;
    fn poll_upstream(&self) -> AdapterResult<Self::UpstreamResult> {
        Box::pin(async { Ok(1) })
    }
    fn poll_cache(&self) -> AdapterResult<CacheState<Self::UpstreamResult>> {
        Box::pin(async { Ok(CacheState::Miss) })
    }
}

#[actix_rt::test]
async fn test_cache_disabled() {
    let settings = CacheSettings {
        cache: Status::Disabled,
        stale: Status::Disabled,
        lock: Status::Disabled,
    };
    let adapter = MockAdapter;
    let initial_state = InitialCacheSettings::from(settings);
    let initial_state = InitialState { adapter, settings: initial_state };
    let result = match initial_state.poll_upstream().await {
        UpstreamPolled::Successful(state) => Ok(state.finish().result()),
        UpstreamPolled::Error(error) => Err(error.finish().result()),
    };
    assert_eq!(result.unwrap(), 1);
}