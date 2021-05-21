use hitbox::dev::MockAdapter;
use hitbox::states::cache_policy::{CachePolicyChecked, CachePolicyNonCacheable};
use hitbox::states::upstream_polled::UpstreamPolledSuccessful;

#[test]
fn test_successful_check_policy_non_cacheable() {
    let adapter = MockAdapter::build().with_upstream_value(42).finish();
    let successful = UpstreamPolledSuccessful {
        adapter,
        result: 42,
    };
    let expected: CachePolicyChecked<MockAdapter<i32>, i32> =
        CachePolicyChecked::NonCacheable(CachePolicyNonCacheable { result: 42 });
    assert!(matches!(successful.check_cache_policy(), expected));
}
