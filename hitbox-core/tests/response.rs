use async_trait::async_trait;
use chrono::Utc;
use hitbox_core::{
    CachePolicy, CacheValue, CacheableResponse, EntityPolicyConfig, Predicate, PredicateError,
    PredicateResult,
};

#[derive(Clone, Debug)]
struct TestResponse {
    #[allow(dead_code)]
    field1: String,
    #[allow(dead_code)]
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

    async fn cache_policy<P>(
        self,
        predicates: P,
        _config: &EntityPolicyConfig,
    ) -> Result<hitbox_core::ResponseCachePolicy<Self>, PredicateError>
    where
        P: hitbox_core::Predicate<Subject = Self::Subject> + Send + Sync,
    {
        match predicates.check(self).await? {
            PredicateResult::Cacheable(cacheable) => match cacheable.into_cached().await {
                CachePolicy::Cacheable(res) => Ok(CachePolicy::Cacheable(CacheValue::new(
                    res,
                    Some(Utc::now()),
                    Some(Utc::now()),
                ))),
                CachePolicy::NonCacheable(res) => Ok(CachePolicy::NonCacheable(res)),
            },
            PredicateResult::NonCacheable(res) => Ok(CachePolicy::NonCacheable(res)),
        }
    }

    async fn into_cached(self) -> CachePolicy<Self::Cached, Self> {
        CachePolicy::Cacheable(self)
    }
    async fn from_cached(cached: Self::Cached) -> Self {
        cached
    }
}

#[derive(Debug)]
struct NeuralPredicate {}

impl NeuralPredicate {
    fn new() -> Self {
        NeuralPredicate {}
    }
}

#[async_trait]
impl Predicate for NeuralPredicate {
    type Subject = TestResponse;

    async fn check(
        &self,
        subject: Self::Subject,
    ) -> Result<PredicateResult<Self::Subject>, PredicateError> {
        Ok(PredicateResult::Cacheable(subject))
    }
}

#[tokio::test]
async fn test_cacheable_result() {
    let response: Result<TestResponse, ()> = Ok(TestResponse::new());
    let policy = response
        .cache_policy(NeuralPredicate::new(), &EntityPolicyConfig::default())
        .await
        .unwrap();
    dbg!(&policy);

    let response: Result<TestResponse, ()> = Err(());
    let policy = response
        .cache_policy(NeuralPredicate::new(), &EntityPolicyConfig::default())
        .await
        .unwrap();
    dbg!(&policy);
}
