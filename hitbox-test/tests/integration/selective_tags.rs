//! Request-tag extraction in `SelectiveCacheFuture` honors the selected
//! config's `TagInvalidation` policy.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use hitbox::concurrency::BroadcastConcurrencyManager;
use hitbox::fsm::SelectiveCacheFuture;
use hitbox::policy::{PolicyConfig, TagInvalidation};
use hitbox::tag::{CacheTag, TagExtractor};
use hitbox::{Config, SelectiveConfig};
use hitbox_core::DisabledOffload;
use hitbox_moka::MokaBackend;
use hitbox_test::fsm::world::{
    ConfigurableRequestPredicate, ConfigurableResponsePredicate, ConfigurableUpstream,
    FixedKeyExtractor, SimpleRequest,
};

/// Request tag extractor that counts how many times it runs.
struct CountingTags(Arc<AtomicUsize>);

#[async_trait]
impl TagExtractor for CountingTags {
    type Subject = SimpleRequest;

    async fn extract_tags(&self, subject: SimpleRequest) -> (SimpleRequest, Vec<CacheTag>) {
        self.0.fetch_add(1, Ordering::SeqCst);
        (subject, vec![CacheTag::new("user=42")])
    }
}

/// Runs one request through a single selective config and returns how many
/// times its request tag extractor ran.
async fn tag_extractions(tag_invalidation: TagInvalidation) -> usize {
    let calls = Arc::new(AtomicUsize::new(0));
    let config = Config::builder()
        .request_predicate(ConfigurableRequestPredicate { cacheable: true })
        .response_predicate(ConfigurableResponsePredicate { cacheable: true })
        .extractor(FixedKeyExtractor)
        .request_tags(CountingTags(calls.clone()))
        .policy(
            PolicyConfig::builder()
                .ttl(Duration::from_secs(60))
                .tag_invalidation(tag_invalidation)
                .build(),
        )
        .build();

    SelectiveCacheFuture::new(
        SelectiveConfig::new(vec![config]),
        Arc::new(MokaBackend::builder().max_entries(10).build()),
        SimpleRequest(1),
        ConfigurableUpstream::new(Arc::new(AtomicUsize::new(0)), 0),
        DisabledOffload,
        Arc::new(BroadcastConcurrencyManager::new()),
    )
    .await;

    calls.load(Ordering::SeqCst)
}

#[tokio::test]
async fn skip_policy_never_runs_request_tag_extractor() {
    assert_eq!(tag_extractions(TagInvalidation::Skip).await, 0);
}

#[tokio::test]
async fn check_policy_runs_request_tag_extractor() {
    assert_eq!(tag_extractions(TagInvalidation::Check).await, 1);
}
