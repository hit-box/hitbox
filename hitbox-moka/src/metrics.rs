//! Moka backend capacity metrics.
//!
//! This module provides metrics for monitoring Moka cache utilization.
//! Enable the `metrics` feature to use these metrics.
//!
//! ## Metrics
//!
//! - `hitbox_moka_entries` - Current number of entries in the cache (gauge)
//! - `hitbox_moka_size_bytes` - Current weighted size in bytes (gauge)
//! - `hitbox_moka_tag_records` - Current number of tag invalidation records in
//!   the dedicated tag store (gauge). The store is unbounded, so this grows with
//!   every distinct tag ever invalidated.
//!
//! All metrics include a `backend` label to distinguish between multiple Moka instances.

#[cfg(feature = "metrics")]
use lazy_static::lazy_static;

#[cfg(feature = "metrics")]
lazy_static! {
    /// Metric name for cache entry count gauge.
    pub static ref MOKA_ENTRIES: &'static str = {
        metrics::describe_gauge!(
            "hitbox_moka_entries",
            "Current number of entries in the Moka cache."
        );
        "hitbox_moka_entries"
    };

    /// Metric name for cache size gauge.
    pub static ref MOKA_SIZE_BYTES: &'static str = {
        metrics::describe_gauge!(
            "hitbox_moka_size_bytes",
            "Current weighted size of the Moka cache in bytes."
        );
        "hitbox_moka_size_bytes"
    };

    /// Metric name for tag invalidation record count gauge.
    pub static ref MOKA_TAG_RECORDS: &'static str = {
        metrics::describe_gauge!(
            "hitbox_moka_tag_records",
            "Current number of tag invalidation records in the Moka tag store."
        );
        "hitbox_moka_tag_records"
    };
}

/// Record current cache capacity metrics.
///
/// Updates the entry count and weighted size gauges for the specified backend.
///
/// # Arguments
///
/// * `backend` - Backend label for metric identification
/// * `entries` - Current number of entries in the cache
/// * `size_bytes` - Current weighted size in bytes
#[cfg(feature = "metrics")]
#[inline]
pub fn record_capacity(backend: &str, entries: u64, size_bytes: u64) {
    metrics::gauge!(*MOKA_ENTRIES, "backend" => backend.to_string()).set(entries as f64);
    metrics::gauge!(*MOKA_SIZE_BYTES, "backend" => backend.to_string()).set(size_bytes as f64);
}

/// Record current cache capacity metrics (no-op when `metrics` feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_capacity(_backend: &str, _entries: u64, _size_bytes: u64) {}

/// Record the current number of tag invalidation records.
///
/// # Arguments
///
/// * `backend` - Backend label for metric identification
/// * `records` - Current number of records in the tag store
#[cfg(feature = "metrics")]
#[inline]
pub fn record_tag_records(backend: &str, records: u64) {
    metrics::gauge!(*MOKA_TAG_RECORDS, "backend" => backend.to_string()).set(records as f64);
}

/// Record the current number of tag invalidation records (no-op when `metrics`
/// feature disabled).
#[cfg(not(feature = "metrics"))]
#[inline]
pub fn record_tag_records(_backend: &str, _records: u64) {}
