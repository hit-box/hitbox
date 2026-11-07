//! Backend metrics for hitbox caching framework.
//!
//! This module provides metrics for cache backend operations.
//! Enable the `metrics` feature to use these metrics.
//!
//! ## Naming Pattern
//!
//! All metrics follow the pattern: `hitbox_backend_{operation}_{metric_type}`
//!
//! - `hitbox_backend_read_*` - read operation metrics
//! - `hitbox_backend_write_*` - write operation metrics
//! - `hitbox_backend_{compress,decompress,serialize,deserialize}_duration_seconds` - processing metrics
//! - `hitbox_backend_{serialize,deserialize}_errors_total` - values the backend's format couldn't
//!   encode or decode

use std::time::Duration;

#[cfg(feature = "metrics")]
use std::cell::RefCell;
#[cfg(feature = "metrics")]
use std::collections::HashMap;
#[cfg(not(feature = "metrics"))]
use std::marker::PhantomData;
#[cfg(feature = "metrics")]
use std::sync::{LazyLock, PoisonError, RwLock};
#[cfg(feature = "metrics")]
use std::time::Instant;

#[cfg(feature = "metrics")]
use lazy_static::lazy_static;
#[cfg(feature = "metrics")]
use metrics::{Counter, Histogram, Key, Label, Level, Metadata};

/// Zero-cost timer for metrics collection.
///
/// When the `metrics` feature is enabled, this captures the start time.
/// When disabled, this is a zero-sized struct with no overhead.
pub struct Timer {
    #[cfg(feature = "metrics")]
    start: Instant,
}

impl Timer {
    /// Create a new timer, capturing the current instant if metrics enabled.
    #[inline]
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "metrics")]
            start: Instant::now(),
        }
    }

    /// Get elapsed duration since timer creation.
    ///
    /// Returns actual elapsed time when metrics enabled, Duration::ZERO otherwise.
    #[inline]
    pub fn elapsed(&self) -> Duration {
        #[cfg(feature = "metrics")]
        {
            self.start.elapsed()
        }
        #[cfg(not(feature = "metrics"))]
        {
            Duration::ZERO
        }
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "metrics")]
lazy_static! {
    // Read operation metrics

    /// Metric name for total read operations counter.
    pub static ref BACKEND_READ_TOTAL: &'static str = {
        metrics::describe_counter!(
            "hitbox_backend_read_total",
            "Total number of cache read operations per backend."
        );
        "hitbox_backend_read_total"
    };

    /// Metric name for read duration histogram.
    pub static ref BACKEND_READ_DURATION: &'static str = {
        metrics::describe_histogram!(
            "hitbox_backend_read_duration_seconds",
            metrics::Unit::Seconds,
            "Duration of raw backend read operations in seconds."
        );
        "hitbox_backend_read_duration_seconds"
    };

    /// Metric name for total bytes read counter.
    pub static ref BACKEND_READ_BYTES: &'static str = {
        metrics::describe_counter!(
            "hitbox_backend_read_bytes_total",
            "Total bytes read from cache per backend."
        );
        "hitbox_backend_read_bytes_total"
    };

    /// Metric name for read errors counter.
    pub static ref BACKEND_READ_ERRORS: &'static str = {
        metrics::describe_counter!(
            "hitbox_backend_read_errors_total",
            "Total number of cache read errors per backend."
        );
        "hitbox_backend_read_errors_total"
    };

    // Write operation metrics

    /// Metric name for total write operations counter.
    pub static ref BACKEND_WRITE_TOTAL: &'static str = {
        metrics::describe_counter!(
            "hitbox_backend_write_total",
            "Total number of cache write operations per backend."
        );
        "hitbox_backend_write_total"
    };

    /// Metric name for write duration histogram.
    pub static ref BACKEND_WRITE_DURATION: &'static str = {
        metrics::describe_histogram!(
            "hitbox_backend_write_duration_seconds",
            metrics::Unit::Seconds,
            "Duration of raw backend write operations in seconds."
        );
        "hitbox_backend_write_duration_seconds"
    };

    /// Metric name for total bytes written counter.
    pub static ref BACKEND_WRITE_BYTES: &'static str = {
        metrics::describe_counter!(
            "hitbox_backend_write_bytes_total",
            "Total bytes written to cache per backend."
        );
        "hitbox_backend_write_bytes_total"
    };

    /// Metric name for write errors counter.
    pub static ref BACKEND_WRITE_ERRORS: &'static str = {
        metrics::describe_counter!(
            "hitbox_backend_write_errors_total",
            "Total number of cache write errors per backend."
        );
        "hitbox_backend_write_errors_total"
    };

    // Processing duration metrics

    /// Metric name for decompression duration histogram.
    pub static ref BACKEND_DECOMPRESS_DURATION: &'static str = {
        metrics::describe_histogram!(
            "hitbox_backend_decompress_duration_seconds",
            metrics::Unit::Seconds,
            "Duration of decompression operations in seconds."
        );
        "hitbox_backend_decompress_duration_seconds"
    };

    /// Metric name for compression duration histogram.
    pub static ref BACKEND_COMPRESS_DURATION: &'static str = {
        metrics::describe_histogram!(
            "hitbox_backend_compress_duration_seconds",
            metrics::Unit::Seconds,
            "Duration of compression operations in seconds."
        );
        "hitbox_backend_compress_duration_seconds"
    };

    /// Metric name for deserialization duration histogram.
    pub static ref BACKEND_DESERIALIZE_DURATION: &'static str = {
        metrics::describe_histogram!(
            "hitbox_backend_deserialize_duration_seconds",
            metrics::Unit::Seconds,
            "Duration of deserialization operations in seconds."
        );
        "hitbox_backend_deserialize_duration_seconds"
    };

    /// Metric name for serialization duration histogram.
    pub static ref BACKEND_SERIALIZE_DURATION: &'static str = {
        metrics::describe_histogram!(
            "hitbox_backend_serialize_duration_seconds",
            metrics::Unit::Seconds,
            "Duration of serialization operations in seconds."
        );
        "hitbox_backend_serialize_duration_seconds"
    };

    // Processing error metrics

    /// Metric name for deserialization errors counter.
    pub static ref BACKEND_DESERIALIZE_ERRORS: &'static str = {
        metrics::describe_counter!(
            "hitbox_backend_deserialize_errors_total",
            "Total number of cached values that failed to deserialize per backend."
        );
        "hitbox_backend_deserialize_errors_total"
    };

    /// Metric name for serialization errors counter.
    pub static ref BACKEND_SERIALIZE_ERRORS: &'static str = {
        metrics::describe_counter!(
            "hitbox_backend_serialize_errors_total",
            "Total number of values that failed to serialize per backend."
        );
        "hitbox_backend_serialize_errors_total"
    };
}

// Backend labels
//
// The metrics macros build a label `Vec` and a `String` on every call. Keys built from a
// `&'static [Label]` allocate nothing, so each backend label is leaked once and reused.
// Leaked labels are never freed (the recorder keeps keys that borrow them), so only the
// first `MAX_INTERNED_LABELS` distinct labels are interned; later ones allocate per call.

/// Most distinct backend labels kept for the process lifetime.
#[cfg(feature = "metrics")]
const MAX_INTERNED_LABELS: usize = 1024;

#[cfg(feature = "metrics")]
static INTERNED_LABELS: LazyLock<RwLock<HashMap<&'static str, &'static [Label]>>> =
    LazyLock::new(Default::default);

#[cfg(feature = "metrics")]
thread_local! {
    // Per-thread copy of `INTERNED_LABELS`, so recording takes no shared lock.
    static LOCAL_LABELS: RefCell<HashMap<&'static str, &'static [Label]>> =
        RefCell::new(HashMap::new());
}

/// Matches what the metrics macros pass from this module.
#[cfg(feature = "metrics")]
static METADATA: Metadata<'static> =
    Metadata::new(module_path!(), Level::INFO, Some(module_path!()));

/// The `backend` label of one operation.
///
/// Look it up once with [`MetricLabels::new`] and pass it to each `record_*` call of the
/// operation.
#[cfg(feature = "metrics")]
#[derive(Clone, Copy)]
pub(crate) enum MetricLabels<'a> {
    Interned(&'static [Label]),
    Uninterned(&'a str),
}

/// The `backend` label of one operation (empty when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[derive(Clone, Copy)]
pub(crate) struct MetricLabels<'a>(PhantomData<&'a str>);

#[cfg(not(feature = "metrics"))]
impl<'a> MetricLabels<'a> {
    #[inline]
    pub(crate) fn new(_backend: &'a str) -> Self {
        Self(PhantomData)
    }
}

#[cfg(feature = "metrics")]
impl<'a> MetricLabels<'a> {
    pub(crate) fn new(backend: &'a str) -> Self {
        if let Some(labels) = LOCAL_LABELS.with_borrow(|local| local.get(backend).copied()) {
            return Self::Interned(labels);
        }
        match intern(backend) {
            Some(labels) => {
                LOCAL_LABELS.with_borrow_mut(|local| local.insert(labels[0].value(), labels));
                Self::Interned(labels)
            }
            None => Self::Uninterned(backend),
        }
    }

    fn key(&self, name: &'static str) -> Key {
        match self {
            Self::Interned(labels) => Key::from_static_labels(name, labels),
            Self::Uninterned(backend) => Key::from_parts(name, vec![backend_label(backend)]),
        }
    }

    fn counter(&self, name: &'static str) -> Counter {
        let key = self.key(name);
        metrics::with_recorder(|recorder| recorder.register_counter(&key, &METADATA))
    }

    fn histogram(&self, name: &'static str) -> Histogram {
        let key = self.key(name);
        metrics::with_recorder(|recorder| recorder.register_histogram(&key, &METADATA))
    }
}

/// Returns the shared labels for `backend`, leaking them on first use, or `None` once
/// `MAX_INTERNED_LABELS` labels exist.
#[cfg(feature = "metrics")]
fn intern(backend: &str) -> Option<&'static [Label]> {
    let interned = INTERNED_LABELS
        .read()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(&labels) = interned.get(backend) {
        return Some(labels);
    }
    if interned.len() >= MAX_INTERNED_LABELS {
        return None;
    }
    drop(interned);

    let mut interned = INTERNED_LABELS
        .write()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(&labels) = interned.get(backend) {
        return Some(labels);
    }
    if interned.len() >= MAX_INTERNED_LABELS {
        return None;
    }
    let labels: &'static [Label] = Box::leak(Box::new([backend_label(backend)]));
    interned.insert(labels[0].value(), labels);
    Some(labels)
}

/// The `backend` label, built the same way for interned and uninterned keys.
#[cfg(feature = "metrics")]
fn backend_label(backend: &str) -> Label {
    Label::new("backend", backend.to_string())
}

// Read metrics

/// Record a read operation with duration.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_read(labels: MetricLabels<'_>, duration: Duration) {
    labels.counter(*BACKEND_READ_TOTAL).increment(1);
    labels
        .histogram(*BACKEND_READ_DURATION)
        .record(duration.as_secs_f64());
}

/// Record a read operation (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_read(_labels: MetricLabels<'_>, _duration: Duration) {}

/// Record bytes read from cache.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_read_bytes(labels: MetricLabels<'_>, bytes: usize) {
    labels.counter(*BACKEND_READ_BYTES).increment(bytes as u64);
}

/// Record bytes read (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_read_bytes(_labels: MetricLabels<'_>, _bytes: usize) {}

/// Record a read error.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_read_error(labels: MetricLabels<'_>) {
    labels.counter(*BACKEND_READ_ERRORS).increment(1);
}

/// Record a read error (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_read_error(_labels: MetricLabels<'_>) {}

// Write metrics

/// Record a write operation with duration.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_write(labels: MetricLabels<'_>, duration: Duration) {
    labels.counter(*BACKEND_WRITE_TOTAL).increment(1);
    labels
        .histogram(*BACKEND_WRITE_DURATION)
        .record(duration.as_secs_f64());
}

/// Record a write operation (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_write(_labels: MetricLabels<'_>, _duration: Duration) {}

/// Record bytes written to cache.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_write_bytes(labels: MetricLabels<'_>, bytes: usize) {
    labels.counter(*BACKEND_WRITE_BYTES).increment(bytes as u64);
}

/// Record bytes written (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_write_bytes(_labels: MetricLabels<'_>, _bytes: usize) {}

/// Record a write error.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_write_error(labels: MetricLabels<'_>) {
    labels.counter(*BACKEND_WRITE_ERRORS).increment(1);
}

/// Record a write error (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_write_error(_labels: MetricLabels<'_>) {}

// Processing metrics

/// Record decompression duration.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_decompress(labels: MetricLabels<'_>, duration: Duration) {
    labels
        .histogram(*BACKEND_DECOMPRESS_DURATION)
        .record(duration.as_secs_f64());
}

/// Record decompression duration (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_decompress(_labels: MetricLabels<'_>, _duration: Duration) {}

/// Record compression duration.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_compress(labels: MetricLabels<'_>, duration: Duration) {
    labels
        .histogram(*BACKEND_COMPRESS_DURATION)
        .record(duration.as_secs_f64());
}

/// Record compression duration (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_compress(_labels: MetricLabels<'_>, _duration: Duration) {}

/// Record deserialization duration.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_deserialize(labels: MetricLabels<'_>, duration: Duration) {
    labels
        .histogram(*BACKEND_DESERIALIZE_DURATION)
        .record(duration.as_secs_f64());
}

/// Record deserialization duration (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_deserialize(_labels: MetricLabels<'_>, _duration: Duration) {}

/// Record serialization duration.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_serialize(labels: MetricLabels<'_>, duration: Duration) {
    labels
        .histogram(*BACKEND_SERIALIZE_DURATION)
        .record(duration.as_secs_f64());
}

/// Record serialization duration (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_serialize(_labels: MetricLabels<'_>, _duration: Duration) {}

/// Record a value that failed to deserialize.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_deserialize_error(labels: MetricLabels<'_>) {
    labels.counter(*BACKEND_DESERIALIZE_ERRORS).increment(1);
}

/// Record a value that failed to deserialize (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_deserialize_error(_labels: MetricLabels<'_>) {}

/// Record a value that failed to serialize.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_serialize_error(labels: MetricLabels<'_>) {
    labels.counter(*BACKEND_SERIALIZE_ERRORS).increment(1);
}

/// Record a value that failed to serialize (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_serialize_error(_labels: MetricLabels<'_>) {}

#[cfg(all(test, feature = "metrics"))]
mod tests {
    use metrics_util::debugging::{DebugValue, DebuggingRecorder};

    use super::*;

    #[test]
    fn interned_and_uninterned_labels_build_the_same_key() {
        // Leaked here, not interned: the cap test may already have filled the shared map.
        let labels: &'static [Label] = Box::leak(Box::new([backend_label("same")]));
        assert_eq!(
            MetricLabels::Interned(labels).key("name"),
            MetricLabels::Uninterned("same").key("name"),
        );
    }

    #[test]
    fn labels_past_the_cap_are_still_recorded() {
        let backends: Vec<String> = (0..MAX_INTERNED_LABELS + 2)
            .map(|i| format!("backend-{i}"))
            .collect();
        let recorder = DebuggingRecorder::new();
        let snapshotter = recorder.snapshotter();
        metrics::with_local_recorder(&recorder, || {
            for backend in backends.iter().chain(&backends) {
                record_read_error(MetricLabels::new(backend));
            }
        });

        let interned = INTERNED_LABELS.read().unwrap().len();
        assert_eq!(interned, MAX_INTERNED_LABELS);

        let counts: HashMap<String, u64> = snapshotter
            .snapshot()
            .into_vec()
            .into_iter()
            .filter_map(|(key, _, _, value)| {
                let backend = key.key().labels().find(|label| label.key() == "backend")?;
                let DebugValue::Counter(count) = value else {
                    return None;
                };
                Some((backend.value().to_string(), count))
            })
            .collect();
        assert_eq!(counts.len(), backends.len());
        assert!(counts.values().all(|&count| count == 2));
    }
}
