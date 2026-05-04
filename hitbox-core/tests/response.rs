use async_trait::async_trait;
use chrono::Utc;
use hitbox_core::tag::{CacheTag, NeutralTagExtractor, TagExtractor};
use hitbox_core::{
    CachePolicy, CacheValue, CacheableResponse, EntityPolicyConfig, Predicate, PredicateResult,
    ResponseCachePolicy,
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

impl CacheableResponse for TestResponse {
    type Cached = Self;
    type Subject = Self;
    type IntoCachedFuture = std::future::Ready<CachePolicy<Self::Cached, Self>>;
    type FromCachedFuture = std::future::Ready<Self>;

    async fn cache_policy<P, TE>(
        self,
        predicates: P,
        tag_extractor: TE,
        _config: &EntityPolicyConfig,
    ) -> (ResponseCachePolicy<Self>, Vec<CacheTag>)
    where
        P: Predicate<Subject = Self::Subject> + Send + Sync,
        TE: TagExtractor<Subject = Self::Subject> + Send + Sync,
    {
        match predicates.check(self).await {
            PredicateResult::Cacheable(cacheable) => {
                let (cacheable, tags) = tag_extractor.extract_tags(cacheable).await;
                match cacheable.into_cached().await {
                    CachePolicy::Cacheable(res) => (
                        CachePolicy::Cacheable(CacheValue::new(
                            res,
                            Some(Utc::now()),
                            Some(Utc::now()),
                        )),
                        tags,
                    ),
                    CachePolicy::NonCacheable(res) => (CachePolicy::NonCacheable(res), tags),
                }
            }
            PredicateResult::NonCacheable(res) => (CachePolicy::NonCacheable(res), Vec::new()),
        }
    }

    fn into_cached(self) -> Self::IntoCachedFuture {
        std::future::ready(CachePolicy::Cacheable(self))
    }

    fn from_cached(cached: Self::Cached) -> Self::FromCachedFuture {
        std::future::ready(cached)
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

    async fn check(&self, subject: Self::Subject) -> PredicateResult<Self::Subject> {
        PredicateResult::Cacheable(subject)
    }
}

#[tokio::test]
async fn test_cacheable_result() {
    let response: Result<TestResponse, ()> = Ok(TestResponse::new());
    let policy = response
        .cache_policy(
            NeuralPredicate::new(),
            NeutralTagExtractor::default(),
            &EntityPolicyConfig::default(),
        )
        .await;
    dbg!(&policy);

    let response: Result<TestResponse, ()> = Err(());
    let policy = response
        .cache_policy(
            NeuralPredicate::new(),
            NeutralTagExtractor::default(),
            &EntityPolicyConfig::default(),
        )
        .await;
    dbg!(&policy);
}
