use hitbox::dev::MockAdapter;
use hitbox::states::cache_polled::{CacheErrorOccurred, CachePolledActual, CachePolledStale};
use hitbox::states::upstream_polled::{
    UpstreamPolled, UpstreamPolledError, UpstreamPolledSuccessful,
};
use hitbox::{CacheError, CachedValue};

#[test]
fn test_cache_actual_debug() {
    let adapter = MockAdapter::build().with_upstream_value(42).finish();
    let actual = CachePolledActual {
        adapter,
        result: CachedValue::new(41, chrono::Utc::now()),
    };
    assert_eq!(format!("{:?}", actual), "CachePolledActual");
}

#[test]
fn test_stale_finish() {
    let adapter: MockAdapter<i32> = MockAdapter::build().with_upstream_error().finish();
    let actual = CachePolledStale {
        adapter,
        result: CachedValue::new(42, chrono::Utc::now()),
    };
    assert_eq!(actual.finish().result.unwrap(), 42)
}
