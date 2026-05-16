//! Tests for verifying metrics correctness.
//!
//! These tests verify that the correct metrics are recorded with correct labels
//! when performing cache operations.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use dashmap::DashMap;
use hitbox_backend::composition::CompositionBackend;
use hitbox_backend::format::{Format, JsonFormat};
use hitbox_backend::{
    Backend, BackendResult, CacheBackend, CacheKeyFormat, Compressor, DeleteStatus,
    PassthroughCompressor, SyncBackend,
};
use hitbox_core::tag::{CacheTag, TagExtractor};
use hitbox_core::{
    BackendLabel, BoxContext, CacheContext, CacheKey, CacheValue, CacheableResponse,
    EntityPolicyConfig, Predicate, Raw, ResponseCachePolicy,
};
use metrics_util::debugging::{DebugValue, DebuggingRecorder};
use metrics_util::{CompositeKey, MetricKind};
use serde::{Deserialize, Serialize};

use super::common::TestOffloadManager;

#[cfg(feature = "rkyv_format")]
use rkyv::{Archive, Serialize as RkyvSerialize};

/// Type alias for snapshot entries
type SnapshotEntry = (
    CompositeKey,
    Option<metrics::Unit>,
    Option<metrics::SharedString>,
    DebugValue,
);

/// Simple in-memory backend for testing using DashMap.
#[derive(Clone)]
struct TestBackend {
    store: Arc<DashMap<CacheKey, CacheValue<Raw>>>,
}

impl TestBackend {
    fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
        }
    }
}

#[async_trait]
impl Backend for TestBackend {
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
        Ok(self.store.get(key).map(|v| v.clone()))
    }

    async fn write(&self, key: &CacheKey, value: CacheValue<Raw>) -> BackendResult<()> {
        self.store.insert(key.clone(), value);
        Ok(())
    }

    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        let existed = self.store.remove(key).is_some();
        Ok(if existed {
            DeleteStatus::Deleted(1)
        } else {
            DeleteStatus::Missing
        })
    }

    fn value_format(&self) -> &dyn Format {
        &JsonFormat
    }

    fn key_format(&self) -> &CacheKeyFormat {
        &CacheKeyFormat::Bitcode
    }

    fn compressor(&self) -> &dyn Compressor {
        &PassthroughCompressor
    }

    fn label(&self) -> BackendLabel {
        BackendLabel::new_static("test")
    }
}

impl CacheBackend for TestBackend {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "rkyv_format",
    derive(Archive, RkyvSerialize, rkyv::Deserialize)
)]
struct TestData {
    id: u32,
    name: String,
}

impl CacheableResponse for TestData {
    type Cached = Self;
    type Subject = Self;
    type IntoCachedFuture = std::future::Ready<hitbox_core::CachePolicy<Self::Cached, Self>>;
    type FromCachedFuture = std::future::Ready<Self>;

    async fn cache_policy<P, TE>(
        self,
        _predicates: P,
        _tag_extractor: Option<TE>,
        _: &EntityPolicyConfig,
    ) -> (ResponseCachePolicy<Self>, Vec<CacheTag>)
    where
        P: Predicate<Subject = Self::Subject> + Send + Sync,
        TE: TagExtractor<Subject = Self::Subject> + Send + Sync,
    {
        todo!()
    }

    fn into_cached(self) -> Self::IntoCachedFuture {
        todo!()
    }

    fn from_cached(_cached: Self::Cached) -> Self::FromCachedFuture {
        todo!()
    }
}

/// Debug: print all metrics in a snapshot.
/// Takes entries directly to avoid draining the snapshot multiple times.
#[allow(dead_code)]
fn debug_entries(entries: &[SnapshotEntry]) {
    println!("=== Snapshot contents ({} entries) ===", entries.len());
    for (key, _, _, value) in entries {
        print!("{:?} {} ", key.kind(), key.key().name());
        for label in key.key().labels() {
            print!("{}={} ", label.key(), label.value());
        }
        println!("= {:?}", value);
    }
    println!("=== End snapshot ===");
}

/// Find a counter in entries by name and label.
fn find_counter_in_entries(
    entries: &[SnapshotEntry],
    name: &str,
    backend_label: &str,
) -> Option<u64> {
    for (key, _, _, value) in entries {
        if key.kind() == MetricKind::Counter && key.key().name() == name {
            for label in key.key().labels() {
                if label.key() == "backend"
                    && label.value() == backend_label
                    && let DebugValue::Counter(v) = *value
                {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// Find a histogram in entries by name and label, return sample count.
fn histogram_count_in_entries(entries: &[SnapshotEntry], name: &str, backend_label: &str) -> usize {
    for (key, _, _, value) in entries {
        if key.kind() == MetricKind::Histogram && key.key().name() == name {
            for label in key.key().labels() {
                if label.key() == "backend"
                    && label.value() == backend_label
                    && let DebugValue::Histogram(v) = value
                {
                    return v.len();
                }
            }
        }
    }
    0
}

// =============================================================================
// Basic Backend Tests
// =============================================================================

/// Test AtomicBucket directly to understand how histogram storage works
#[test]
fn test_atomic_bucket_directly() {
    use metrics::Key;
    use metrics_util::registry::{AtomicStorage, Registry};

    // Create a registry directly
    let registry: Registry<Key, AtomicStorage> = Registry::atomic();
    let key = Key::from_name("test_histogram");

    // Get or create histogram and push some values
    registry.get_or_create_histogram(&key, |h| {
        h.push(1.5);
        h.push(2.0);
        h.push(3.0);
    });

    // Now get the histogram handles and read values using data()
    let histograms = registry.get_histogram_handles();
    let histogram = histograms.get(&key).expect("histogram should exist");

    // Use data() which is the read method that bucket tests use
    let data = histogram.data();
    println!("Histogram data via data(): {:?}", data);
    assert_eq!(data.len(), 3, "Should have 3 samples via data()");

    // Now test clear_with which is what DebuggingRecorder uses
    let mut values = Vec::new();
    histogram.clear_with(|xs| {
        values.extend(xs.iter().copied());
    });
    println!("Histogram data via clear_with: {:?}", values);

    // After clear_with, data should be empty
    let data_after_clear = histogram.data();
    println!("Histogram data after clear_with: {:?}", data_after_clear);
}

/// Test full chain: DebuggingRecorder -> register_histogram -> record -> snapshot
#[test]
fn test_debugging_recorder_full_chain() {
    use metrics::{HistogramFn, Key, Level, Metadata, Recorder};
    use metrics_util::registry::{AtomicStorage, Registry};

    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();

    // Directly call register_histogram on the recorder (not through macros)
    let key = Key::from_name("direct_histogram");
    let metadata = Metadata::new("test", Level::INFO, None);
    let histogram = recorder.register_histogram(&key, &metadata);

    println!("Got histogram handle: {:?}", histogram);

    // Record some values using the Histogram handle
    histogram.record(1.5);
    histogram.record(2.0);
    histogram.record(3.0);

    // Get handles directly from registry to check what's there
    // This mimics what Snapshotter.snapshot() does
    println!("\n=== Checking registry directly after histogram.record() ===");

    // Take snapshot - NOTE: clear_with DRAINS the data, so only take ONE snapshot!
    let snapshot = snapshotter.snapshot();
    let entries = snapshot.into_vec();
    println!("Snapshot: {} entries", entries.len());
    for (key, _, _, value) in entries {
        println!("  {:?} {} = {:?}", key.kind(), key.key().name(), value);
    }

    // Now test: use HistogramFn trait directly on an Arc<AtomicBucket>
    println!("\n=== Testing HistogramFn trait on Arc ===");
    let registry: Registry<Key, AtomicStorage> = Registry::atomic();
    let test_key = Key::from_name("test_histogramfn");

    let arc_bucket = registry.get_or_create_histogram(&test_key, |h| h.clone());
    println!(
        "Got Arc<AtomicBucket>, data before record: {:?}",
        arc_bucket.data()
    );

    // Call record() through the HistogramFn trait (what Histogram::record does)
    HistogramFn::record(&arc_bucket, 1.0);
    HistogramFn::record(&arc_bucket, 2.0);
    println!(
        "After HistogramFn::record calls, data: {:?}",
        arc_bucket.data()
    );

    // Now create a metrics::Histogram from this arc and test
    println!("\n=== Testing metrics::Histogram wrapper ===");
    let metrics_histogram = metrics::Histogram::from_arc(arc_bucket.clone());
    metrics_histogram.record(100.0);
    metrics_histogram.record(200.0);
    println!(
        "After Histogram::record calls, arc_bucket.data: {:?}",
        arc_bucket.data()
    );

    // The KEY question: when DebuggingRecorder.register_histogram creates a Histogram,
    // does calling .record() on that Histogram write to the SAME bucket as in the registry?
    println!("\n=== Verifying DebuggingRecorder registry storage ===");
    let recorder2 = DebuggingRecorder::new();
    let key2 = Key::from_name("verify_histogram");
    let metadata2 = Metadata::new("test", Level::INFO, None);

    // Get histogram and record
    let h = recorder2.register_histogram(&key2, &metadata2);
    h.record(42.0);
    h.record(43.0);

    // Now get the histogram handles directly from registry via another registration
    // and use a closure that reads the data
    let _data_in_registry = recorder2.register_histogram(&key2, &metadata2);

    // The issue: Histogram doesn't expose the inner Arc, so we can't check it directly.
    // But we CAN check via snapshot:
    let snap = recorder2.snapshotter().snapshot();
    for (k, _, _, v) in snap.into_vec() {
        println!("After record: {:?} {} = {:?}", k.kind(), k.key().name(), v);
    }

    // HYPOTHESIS: Maybe the issue is that Histogram::from_arc creates a NEW Arc
    // that wraps the input Arc, causing double indirection?
    // Let's test this explicitly:
    println!("\n=== Testing potential double-Arc issue ===");

    // The inner of metrics::Histogram is Arc<dyn HistogramFn>
    // When we call from_arc(Arc<AtomicBucket>), does it become Arc<Arc<AtomicBucket>>?

    // Actually, from_arc takes Arc<F> and stores it as Arc<dyn HistogramFn>
    // This is an UNSIZING coercion, not wrapping in another Arc.
    // Arc<AtomicBucket<f64>> -> Arc<dyn HistogramFn> - this should work via CoerceUnsized

    // Let me check if the histogram inner is actually Some by recording and checking snapshot
}

#[test]
fn test_basic_backend_write_metrics() {
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();

    metrics::with_local_recorder(&recorder, || {
        // Use single-threaded runtime so all async code runs on same thread as local recorder
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let backend = TestBackend::new();
            let key = CacheKey::from_str("test_key", "");
            let data = TestData {
                id: 42,
                name: "test".to_string(),
            };
            let value =
                CacheValue::new(data, Some(Utc::now() + chrono::Duration::seconds(60)), None);

            // Perform a cache write
            let mut ctx: BoxContext = CacheContext::default().boxed();
            backend
                .set::<TestData>(&key, &value, &mut ctx)
                .await
                .unwrap();
        })
    });

    // Take ONE snapshot - clear_with drains data, so only one snapshot works
    let entries = snapshotter.snapshot().into_vec();

    // Debug: print what's in the snapshot
    debug_entries(&entries);

    // Verify all write-related metrics
    assert_eq!(
        find_counter_in_entries(&entries, "hitbox_backend_write_total", "test"),
        Some(1),
        "write_total counter should be 1"
    );

    assert!(
        find_counter_in_entries(&entries, "hitbox_backend_write_bytes_total", "test").unwrap() > 0,
        "write_bytes_total should be > 0"
    );

    assert_eq!(
        histogram_count_in_entries(&entries, "hitbox_backend_write_duration_seconds", "test"),
        1,
        "write_duration should have 1 sample"
    );

    assert_eq!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_serialize_duration_seconds",
            "test"
        ),
        1,
        "serialize_duration should have 1 sample"
    );

    assert_eq!(
        histogram_count_in_entries(&entries, "hitbox_backend_compress_duration_seconds", "test"),
        1,
        "compress_duration should have 1 sample"
    );
}

#[test]
fn test_basic_backend_read_metrics() {
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();

    metrics::with_local_recorder(&recorder, || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let backend = TestBackend::new();
            let key = CacheKey::from_str("test_key", "");
            let data = TestData {
                id: 42,
                name: "test".to_string(),
            };
            let value = CacheValue::new(
                data.clone(),
                Some(Utc::now() + chrono::Duration::seconds(60)),
                None,
            );

            // First write data to have something to read
            let mut ctx: BoxContext = CacheContext::default().boxed();
            backend
                .set::<TestData>(&key, &value, &mut ctx)
                .await
                .unwrap();

            // Perform a cache read (hit)
            let result: Option<CacheValue<TestData>> =
                backend.get::<TestData>(&key, &mut ctx).await.unwrap();
            assert!(result.is_some());
            assert_eq!(*result.unwrap().data(), data);
        })
    });

    // Take ONE snapshot
    let entries = snapshotter.snapshot().into_vec();

    // Verify all read-related metrics
    assert_eq!(
        find_counter_in_entries(&entries, "hitbox_backend_read_total", "test"),
        Some(1),
        "read_total counter should be 1"
    );

    assert!(
        find_counter_in_entries(&entries, "hitbox_backend_read_bytes_total", "test").unwrap() > 0,
        "read_bytes_total should be > 0"
    );

    assert_eq!(
        histogram_count_in_entries(&entries, "hitbox_backend_read_duration_seconds", "test"),
        1,
        "read_duration should have 1 sample"
    );

    assert_eq!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_deserialize_duration_seconds",
            "test"
        ),
        1,
        "deserialize_duration should have 1 sample"
    );

    assert_eq!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_decompress_duration_seconds",
            "test"
        ),
        1,
        "decompress_duration should have 1 sample"
    );
}

// =============================================================================
// Composition Backend Tests
// =============================================================================

#[test]
fn test_composition_backend_write_metrics() {
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();

    metrics::with_local_recorder(&recorder, || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let l1 = TestBackend::new();
            let l2 = TestBackend::new();

            let composition = CompositionBackend::new(l1, l2, TestOffloadManager).label("comp");

            let key = CacheKey::from_str("test_key", "");
            let data = TestData {
                id: 42,
                name: "test".to_string(),
            };
            let value =
                CacheValue::new(data, Some(Utc::now() + chrono::Duration::seconds(60)), None);

            // Perform a cache write
            let mut ctx: BoxContext = CacheContext::default().boxed();
            composition
                .set::<TestData>(&key, &value, &mut ctx)
                .await
                .unwrap();
        })
    });

    // Take ONE snapshot
    let entries = snapshotter.snapshot().into_vec();

    // Debug
    debug_entries(&entries);

    // CompositionBackend now uses composed labels: "{composition_name}.{backend_name}"
    // Both L1 and L2 are TestBackend("test"), so both get label "comp.test"
    // Write to both L1 and L2 = 2 writes with same label
    assert_eq!(
        find_counter_in_entries(&entries, "hitbox_backend_write_total", "comp.test"),
        Some(2),
        "write_total should be 2 (one per layer)"
    );

    assert!(
        find_counter_in_entries(&entries, "hitbox_backend_write_bytes_total", "comp.test").unwrap()
            > 0,
        "write_bytes_total should be > 0"
    );

    // 2 samples: one from L1 write, one from L2 write
    assert_eq!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_write_duration_seconds",
            "comp.test"
        ),
        2,
        "write_duration should have 2 samples (one per layer)"
    );

    // Serialization: with same-format optimization, only 1 serialize call for both layers
    // (L1 serialized data is reused for L2 since formats are equal)
    assert!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_serialize_duration_seconds",
            "comp.test"
        ) >= 1,
        "serialize_duration should be recorded"
    );

    // Compression: 2 samples (one per layer, different compressors possible)
    assert_eq!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_compress_duration_seconds",
            "comp.test"
        ),
        2,
        "compress_duration should have 2 samples (one per layer)"
    );
}

#[test]
fn test_composition_backend_read_metrics() {
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();

    metrics::with_local_recorder(&recorder, || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let l1 = TestBackend::new();
            let l2 = TestBackend::new();

            let composition = CompositionBackend::new(l1, l2, TestOffloadManager).label("comp");

            let key = CacheKey::from_str("test_key", "");
            let data = TestData {
                id: 42,
                name: "test".to_string(),
            };
            let value = CacheValue::new(
                data.clone(),
                Some(Utc::now() + chrono::Duration::seconds(60)),
                None,
            );

            // First write data
            let mut ctx: BoxContext = CacheContext::default().boxed();
            composition
                .set::<TestData>(&key, &value, &mut ctx)
                .await
                .unwrap();

            // Then read it back
            let result: Option<CacheValue<TestData>> =
                composition.get::<TestData>(&key, &mut ctx).await.unwrap();
            assert!(result.is_some());
            assert_eq!(*result.unwrap().data(), data);
        })
    });

    // Take ONE snapshot
    let entries = snapshotter.snapshot().into_vec();

    // Debug
    debug_entries(&entries);

    // CompositionBackend now uses composed labels: "comp.test"
    // The set() wrote to both layers (2 writes), and read() hit L1 (1 read).

    // Verify read metrics (L1 hit means only 1 read)
    assert_eq!(
        find_counter_in_entries(&entries, "hitbox_backend_read_total", "comp.test"),
        Some(1),
        "read_total should be 1 (L1 hit)"
    );

    assert!(
        find_counter_in_entries(&entries, "hitbox_backend_read_bytes_total", "comp.test").unwrap()
            > 0,
        "read_bytes_total should be > 0"
    );

    assert_eq!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_read_duration_seconds",
            "comp.test"
        ),
        1,
        "read_duration should have 1 sample"
    );

    // Verify deserialization and decompression metrics
    assert_eq!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_deserialize_duration_seconds",
            "comp.test"
        ),
        1,
        "deserialize_duration should have 1 sample"
    );

    assert_eq!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_decompress_duration_seconds",
            "comp.test"
        ),
        1,
        "decompress_duration should have 1 sample"
    );
}

// =============================================================================
// Dyn Backend Tests (trait objects)
// =============================================================================

#[test]
fn test_dyn_composition_backend_write_metrics() {
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();

    metrics::with_local_recorder(&recorder, || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            // Use Arc<SyncBackend> for inner backends (required for Clone)
            let l1: Arc<SyncBackend> = Arc::new(TestBackend::new());
            let l2: Arc<SyncBackend> = Arc::new(TestBackend::new());

            let composition = CompositionBackend::new(l1, l2, TestOffloadManager).label("dyncomp");

            let key = CacheKey::from_str("test_key", "");
            let data = TestData {
                id: 42,
                name: "test".to_string(),
            };
            let value =
                CacheValue::new(data, Some(Utc::now() + chrono::Duration::seconds(60)), None);

            // Perform a cache write via trait object
            let mut ctx: BoxContext = CacheContext::default().boxed();
            composition
                .set::<TestData>(&key, &value, &mut ctx)
                .await
                .unwrap();
        })
    });

    // Take ONE snapshot
    let entries = snapshotter.snapshot().into_vec();

    // Debug
    debug_entries(&entries);

    // With Arc<SyncBackend>, both L1 and L2 have composed label "dyncomp.test"
    // Write to both layers = 2 writes
    assert_eq!(
        find_counter_in_entries(&entries, "hitbox_backend_write_total", "dyncomp.test"),
        Some(2),
        "write_total should be 2 (one per layer)"
    );

    assert!(
        find_counter_in_entries(&entries, "hitbox_backend_write_bytes_total", "dyncomp.test")
            .unwrap()
            > 0,
        "write_bytes_total should be > 0"
    );

    assert_eq!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_write_duration_seconds",
            "dyncomp.test"
        ),
        2,
        "write_duration should have 2 samples (one per layer)"
    );

    // Serialization: with same-format optimization, only 1 serialize call
    assert!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_serialize_duration_seconds",
            "dyncomp.test"
        ) >= 1,
        "serialize_duration should be recorded"
    );

    // Compression: 2 samples (one per layer)
    assert_eq!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_compress_duration_seconds",
            "dyncomp.test"
        ),
        2,
        "compress_duration should have 2 samples (one per layer)"
    );
}

#[test]
fn test_dyn_composition_backend_read_metrics() {
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();

    metrics::with_local_recorder(&recorder, || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            // Use Arc<SyncBackend> for inner backends (required for Clone)
            let l1: Arc<SyncBackend> = Arc::new(TestBackend::new());
            let l2: Arc<SyncBackend> = Arc::new(TestBackend::new());

            let composition = CompositionBackend::new(l1, l2, TestOffloadManager).label("dyncomp");

            let key = CacheKey::from_str("test_key", "");
            let data = TestData {
                id: 42,
                name: "test".to_string(),
            };
            let value = CacheValue::new(
                data.clone(),
                Some(Utc::now() + chrono::Duration::seconds(60)),
                None,
            );

            // First write data
            let mut ctx: BoxContext = CacheContext::default().boxed();
            composition
                .set::<TestData>(&key, &value, &mut ctx)
                .await
                .unwrap();

            // Then read it back via trait object
            let result: Option<CacheValue<TestData>> =
                composition.get::<TestData>(&key, &mut ctx).await.unwrap();
            assert!(result.is_some());
            assert_eq!(*result.unwrap().data(), data);
        })
    });

    // Take ONE snapshot
    let entries = snapshotter.snapshot().into_vec();

    // Verify read metrics with composed label
    assert_eq!(
        find_counter_in_entries(&entries, "hitbox_backend_read_total", "dyncomp.test"),
        Some(1),
        "read_total should be 1"
    );

    assert!(
        find_counter_in_entries(&entries, "hitbox_backend_read_bytes_total", "dyncomp.test")
            .unwrap()
            > 0,
        "read_bytes_total should be > 0"
    );

    assert_eq!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_read_duration_seconds",
            "dyncomp.test"
        ),
        1,
        "read_duration should have 1 sample"
    );

    assert!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_deserialize_duration_seconds",
            "dyncomp.test"
        ) >= 1,
        "deserialize_duration should be recorded"
    );

    assert!(
        histogram_count_in_entries(
            &entries,
            "hitbox_backend_decompress_duration_seconds",
            "dyncomp.test"
        ) >= 1,
        "decompress_duration should be recorded"
    );
}
