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

use std::time::Duration;

#[cfg(feature = "metrics")]
use std::time::Instant;

#[cfg(feature = "metrics")]
use lazy_static::lazy_static;

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

    /// Metric name for value-envelope decode failures counter.
    pub static ref BACKEND_DECODE_ERRORS: &'static str = {
        metrics::describe_counter!(
            "hitbox_backend_decode_errors_total",
            "Total value-envelope decode failures treated as cache misses, per backend."
        );
        "hitbox_backend_decode_errors_total"
    };
}

// Read metrics

/// Record a read operation with duration.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_read(backend: &str, duration: Duration) {
    metrics::counter!(*BACKEND_READ_TOTAL, "backend" => backend.to_string()).increment(1);
    metrics::histogram!(*BACKEND_READ_DURATION, "backend" => backend.to_string())
        .record(duration.as_secs_f64());
}

/// Record a read operation (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_read(_backend: &str, _duration: Duration) {}

/// Record bytes read from cache.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_read_bytes(backend: &str, bytes: usize) {
    metrics::counter!(*BACKEND_READ_BYTES, "backend" => backend.to_string())
        .increment(bytes as u64);
}

/// Record bytes read (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_read_bytes(_backend: &str, _bytes: usize) {}

/// Record a read error.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_read_error(backend: &str) {
    metrics::counter!(*BACKEND_READ_ERRORS, "backend" => backend.to_string()).increment(1);
}

/// Record a read error (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_read_error(_backend: &str) {}

// Write metrics

/// Record a write operation with duration.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_write(backend: &str, duration: Duration) {
    metrics::counter!(*BACKEND_WRITE_TOTAL, "backend" => backend.to_string()).increment(1);
    metrics::histogram!(*BACKEND_WRITE_DURATION, "backend" => backend.to_string())
        .record(duration.as_secs_f64());
}

/// Record a write operation (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_write(_backend: &str, _duration: Duration) {}

/// Record bytes written to cache.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_write_bytes(backend: &str, bytes: usize) {
    metrics::counter!(*BACKEND_WRITE_BYTES, "backend" => backend.to_string())
        .increment(bytes as u64);
}

/// Record bytes written (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_write_bytes(_backend: &str, _bytes: usize) {}

/// Record a write error.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_write_error(backend: &str) {
    metrics::counter!(*BACKEND_WRITE_ERRORS, "backend" => backend.to_string()).increment(1);
}

/// Record a write error (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_write_error(_backend: &str) {}

// Processing metrics

/// Record decompression duration.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_decompress(backend: &str, duration: Duration) {
    metrics::histogram!(*BACKEND_DECOMPRESS_DURATION, "backend" => backend.to_string())
        .record(duration.as_secs_f64());
}

/// Record decompression duration (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_decompress(_backend: &str, _duration: Duration) {}

/// Record compression duration.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_compress(backend: &str, duration: Duration) {
    metrics::histogram!(*BACKEND_COMPRESS_DURATION, "backend" => backend.to_string())
        .record(duration.as_secs_f64());
}

/// Record compression duration (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_compress(_backend: &str, _duration: Duration) {}

/// Record deserialization duration.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_deserialize(backend: &str, duration: Duration) {
    metrics::histogram!(*BACKEND_DESERIALIZE_DURATION, "backend" => backend.to_string())
        .record(duration.as_secs_f64());
}

/// Record deserialization duration (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_deserialize(_backend: &str, _duration: Duration) {}

/// Record serialization duration.
#[cfg(feature = "metrics")]
#[inline]
pub fn record_serialize(backend: &str, duration: Duration) {
    metrics::histogram!(*BACKEND_SERIALIZE_DURATION, "backend" => backend.to_string())
        .record(duration.as_secs_f64());
}

/// Record serialization duration (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_serialize(_backend: &str, _duration: Duration) {}

// Decode metrics

/// Record a value-envelope decode failure (treated as a cache miss).
#[cfg(feature = "metrics")]
#[inline]
pub fn record_decode_error(backend: &str) {
    metrics::counter!(*BACKEND_DECODE_ERRORS, "backend" => backend.to_string()).increment(1);
}

/// Record a decode failure (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_decode_error(_backend: &str) {}
