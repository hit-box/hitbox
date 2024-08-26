use async_trait::async_trait;
use chrono::Utc;
use hitbox_core::{CachePolicy, CacheableResponse, CachedValue, Predicate, PredicateResult};

#[derive(Clone, Debug)]
struct TestResponse {
    field1: String,
    field2: u8,
}

impl TestResponse {
    pub fn new() -> Self {
        Self {
            field1: "nope".to_owned(),
            field2: 42,
        }
    }
}

#[async_trait]
impl CacheableResponse for TestResponse {
    type Cached = Self;
    type Subject = Self;

    async fn cache_policy<P>(self, predicates: P) -> hitbox_core::ResponseCachePolicy<Self>
    where
        P: hitbox_core::Predicate<Subject = Self::Subject> + Send + Sync,
    {
        match predicates.check(self).await {
            PredicateResult::Cacheable(cacheable) => match cacheable.into_cached().await {
                CachePolicy::Cacheable(res) => {
                    CachePolicy::Cacheable(CachedValue::new(res, Utc::now()))
                }
                CachePolicy::NonCacheable(res) => CachePolicy::NonCacheable(res),
            },
            PredicateResult::NonCacheable(res) => CachePolicy::NonCacheable(res),
        }
    }

    async fn into_cached(self) -> CachePolicy<Self::Cached, Self> {
        CachePolicy::Cacheable(self)
    }
    async fn from_cached(cached: Self::Cached) -> Self {
        cached
    }
}

struct NeuralPredicate {}

impl NeuralPredicate {
    fn new() -> Self {
        NeuralPredicate {}
    }
}

#[async_trait]
impl Predicate for NeuralPredicate {
    type Subject = TestResponse;

    async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject> {
        PredicateResult::Cacheable(subject)
    }
}

#[tokio::test]
async fn test_cacheable_result() {
    let response: Result<TestResponse, ()> = Ok(TestResponse::new());
    let policy = response.cache_policy(NeuralPredicate::new()).await;
    dbg!(&policy);

    let response: Result<TestResponse, ()> = Err(());
    let policy = response.cache_policy(NeuralPredicate::new()).await;
    dbg!(&policy);
}
