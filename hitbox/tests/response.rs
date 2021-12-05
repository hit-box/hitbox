use hitbox::{CachePolicy, CacheableResponse};

#[test]
fn test_optinal_cacheable_response() {
    let maybe1: Option<i32> = Some(12);
    let maybe2: Option<i32> = None;

    assert!(matches!(maybe1.cache_policy(), CachePolicy::Cacheable(12)));
    assert!(matches!(
        maybe2.cache_policy(),
        CachePolicy::NonCacheable(())
    ));

    assert!(matches!(
        maybe1.into_cache_policy(),
        CachePolicy::Cacheable(12)
    ));
    assert!(matches!(
        maybe2.into_cache_policy(),
        CachePolicy::NonCacheable(None)
    ));

    assert!(matches!(Option::from_cached(12), Some(12)));
}

#[test]
fn test_result_cacheable_response() {
    let result1: Result<i32, &str> = Ok(12);
    let result2: Result<i32, &str> = Err("error");

    assert!(matches!(result1.cache_policy(), CachePolicy::Cacheable(12)));
    assert!(matches!(
        result2.cache_policy(),
        CachePolicy::NonCacheable(())
    ));

    assert!(matches!(
        result1.into_cache_policy(),
        CachePolicy::Cacheable(12)
    ));
    assert!(matches!(
        result2.into_cache_policy(),
        CachePolicy::NonCacheable(Err("error"))
    ));

    let result3: Result<i32, &str> = Result::from_cached(12);
    assert!(matches!(result3, Ok(12)));
}
