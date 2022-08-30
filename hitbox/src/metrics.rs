//! Metrics declaration and initialization.
use lazy_static::lazy_static;

lazy_static! {
    /// Track number of cache hit events.
    pub static ref CACHE_HIT_COUNTER: &'static str = {
        metrics::describe_counter!(
            "cache_hit_count",
            "Total number of cache hit events by message and actor."
        );
        "cache_hit_count"
    };
    /// Track number of cache miss events.
    pub static ref CACHE_MISS_COUNTER: &'static str = {
        metrics::describe_counter!(
            "cache_miss_count",
            "Total number of cache miss events by message and actor."
        );
        "cache_miss_count"
    };
    /// Track number of cache stale events.
    pub static ref CACHE_STALE_COUNTER: &'static str = {
        metrics::describe_counter!(
            "cache_stale_count",
            "Total number of cache stale events by message and actor."
        );
        "cache_stale_count"
    };
    /// Metric of upstream message handling timings.
    pub static ref CACHE_UPSTREAM_HANDLING_HISTOGRAM: &'static str = {
        metrics::describe_histogram!(
            "cache_upstream_message_handling_duration_seconds",
            metrics::Unit::Seconds,
            "Cache upstream actor message handling latencies in seconds."
        );
        "cache_upstream_message_handling_duration_seconds"
    };
}
