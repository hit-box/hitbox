//! Metrics declaration and initialization.
use lazy_static::lazy_static;
use prometheus::{register_histogram_vec, register_int_counter_vec, HistogramVec, IntCounterVec};

lazy_static! {
    /// Track number of cache hit events.
    pub static ref CACHE_HIT_COUNTER: IntCounterVec = register_int_counter_vec!(
        "cache_hit_count",
        "Total number of cache hit events by message and actor.",
        &["message", "upsream"]
    ).unwrap();

    /// Track number of cache miss events.
    pub static ref CACHE_MISS_COUNTER: IntCounterVec = register_int_counter_vec!(
        "cache_miss_count",
        "Total number of cache miss events by message and actor.",
        &["message", "upstream"]
    ).unwrap();

    /// Track number of cache stale events.
    pub static ref CACHE_STALE_COUNTER: IntCounterVec = register_int_counter_vec!(
        "cache_stale_count",
        "Total number of cache stale events by message and actor.",
        &["message", "upstream"]
    ).unwrap();

    /// Metric of actor message handling timings.
    pub static ref CACHE_UPSTREAM_HANDLING_HISTOGRAM: HistogramVec = register_histogram_vec!(
        "cache_upstream_message_handling_duration_seconds",
        "Cache upstream actor message handling latencies in seconds.",
        &["message", "upstream"]
    ).unwrap();
}
