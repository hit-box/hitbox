#[cfg(all(test, feature = "cache-metrics"))]
mod tests {
    use hitbox::dev::MockAdapter;
    use hitbox::metrics::CACHE_HIT_COUNTER;
    use hitbox::settings::Status;
    use hitbox::states::initial::Initial;
    use metrics::{Counter, Gauge, Histogram, Key, KeyName, Label, Recorder, Unit};
    use metrics_util::registry::{AtomicStorage, Registry};
    use std::sync::{atomic::Ordering, Arc};

    static LABELS: [Label; 2] = [
        Label::from_static_parts("upstream", "MockAdapter"),
        Label::from_static_parts("message", "MockMessage"),
    ];

    struct MockRecorder {
        pub registry: Arc<Registry<Key, AtomicStorage>>,
    }

    impl Clone for MockRecorder {
        fn clone(&self) -> Self {
            Self {
                registry: self.registry.clone(),
            }
        }
    }

    impl MockRecorder {
        pub fn new() -> Self {
            Self {
                registry: Arc::new(Registry::atomic()),
            }
        }
    }

    impl Recorder for MockRecorder {
        fn describe_counter(&self, _: KeyName, _: Option<Unit>, _: &'static str) {}

        fn describe_gauge(&self, _: KeyName, _: Option<Unit>, _: &'static str) {}

        fn describe_histogram(&self, _: KeyName, _: Option<Unit>, _: &'static str) {}

        fn register_counter(&self, key: &Key) -> Counter {
            self.registry
                .get_or_create_counter(key, |c| c.clone().into())
        }

        fn register_gauge(&self, key: &Key) -> Gauge {
            self.registry.get_or_create_gauge(key, |c| c.clone().into())
        }

        fn register_histogram(&self, key: &Key) -> Histogram {
            self.registry
                .get_or_create_histogram(key, |c| c.clone().into())
        }
    }

    #[tokio::test]
    async fn test_hit_counter() {
        let recorder = MockRecorder::new();
        let handler = recorder.clone();
        metrics::set_boxed_recorder(Box::new(recorder)).unwrap();
        let settings = hitbox::settings::CacheSettings {
            cache: Status::Enabled,
            stale: Status::Disabled,
            lock: Status::Disabled,
        };
        let adapter = MockAdapter::build()
            .with_upstream_value(42)
            .with_cache_actual(41)
            .finish();
        let initial_state = Initial::new(settings.clone(), adapter.clone());
        let _ = initial_state.transitions().await.unwrap();

        let metrics_key = Key::from_parts(CACHE_HIT_COUNTER.as_ref(), LABELS.to_vec());
        let counters = handler.registry.get_counter_handles();
        let counter = counters.get(&metrics_key);
        assert!(counter.is_some());
        assert_eq!(counter.unwrap().load(Ordering::Acquire), 1);
    }
}
