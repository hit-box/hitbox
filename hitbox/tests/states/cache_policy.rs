use hitbox::dev::MockAdapter;
use hitbox::states::cache_policy::{CachePolicyCacheable, CachePolicyNonCacheable};
use hitbox::CacheError;

#[test]
fn test_cacheable_debug() {
    let adapter = MockAdapter::build().with_upstream_value(42).finish();
    let cacheable = CachePolicyCacheable {
        adapter,
        result: 42,
    };
    assert_eq!(format!("{:?}", cacheable), "CachePolicyCacheable");
}

#[test]
fn test_non_cacheable_debug() {
    let non_cacheable = CachePolicyNonCacheable { result: 42 };
    assert_eq!(format!("{:?}", non_cacheable), "CachePolicyNonCacheable");
}

#[test]
fn test_non_cacheable_finish() {
    let non_cacheable = CachePolicyNonCacheable { result: 42 };
    assert_eq!(non_cacheable.finish().result.unwrap(), 42);
}
