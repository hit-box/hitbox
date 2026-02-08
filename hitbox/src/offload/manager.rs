//! OffloadManager implementation for background task execution.

use std::future::Future;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use dashmap::DashMap;
use hitbox_core::OffloadKey as CoreOffloadKey;
use smol_str::SmolStr;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tracing::{Instrument, debug, info_span, warn};

use crate::CacheKey;

use super::policy::{OffloadConfig, TimeoutPolicy};

#[cfg(feature = "metrics")]
use crate::metrics::{
    OFFLOAD_TASK_DURATION, OFFLOAD_TASKS_ACTIVE, OFFLOAD_TASKS_COMPLETED,
    OFFLOAD_TASKS_DEDUPLICATED, OFFLOAD_TASKS_SPAWNED, OFFLOAD_TASKS_TIMEOUT,
};

/// Key for identifying offloaded tasks.
///
/// # Deprecation
///
/// This type will be removed in version 0.3. Use [`hitbox_core::OffloadKey`] instead.
#[deprecated(
    since = "0.2.1",
    note = "use `hitbox_core::OffloadKey` instead, will be removed in 0.3"
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OffloadKey {
    /// Key derived from cache key (enables deduplication for cache operations).
    Cache(CacheKey),
    /// Auto-generated key for non-cache tasks with a kind prefix.
    Generated {
        /// Kind of the task (e.g., "revalidate", "warmup", "cleanup").
        kind: SmolStr,
        /// Unique identifier within the kind.
        id: u64,
    },
}

#[allow(deprecated)]
impl OffloadKey {
    /// Returns the key type for metrics labels.
    ///
    /// For `Cache` keys returns "cache".
    /// For `Generated` keys returns the kind.
    pub fn key_type(&self) -> SmolStr {
        match self {
            Self::Cache(_) => SmolStr::new_static("cache"),
            Self::Generated { kind, .. } => kind.clone(),
        }
    }
}

#[allow(deprecated)]
impl From<CacheKey> for OffloadKey {
    fn from(key: CacheKey) -> Self {
        Self::Cache(key)
    }
}

/// Handle to a spawned offload task.
#[derive(Debug)]
pub struct OffloadHandle {
    handle: JoinHandle<()>,
}

impl OffloadHandle {
    /// Check if the task is finished.
    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    /// Abort the task.
    pub fn abort(&self) {
        self.handle.abort();
    }
}

/// Internal state shared across clones.
#[derive(Debug)]
struct OffloadManagerInner {
    config: OffloadConfig,
    tasks: DashMap<CoreOffloadKey, OffloadHandle>,
    key_counter: AtomicU64,
    task_completed: Notify,
}

/// Manager for offloading tasks to background execution.
///
/// Supports task deduplication, timeout policies, and metrics collection.
#[derive(Clone, Debug)]
pub struct OffloadManager {
    inner: Arc<OffloadManagerInner>,
}

impl OffloadManager {
    /// Create a new OffloadManager with the given configuration.
    pub fn new(config: OffloadConfig) -> Self {
        Self {
            inner: Arc::new(OffloadManagerInner {
                config,
                tasks: DashMap::new(),
                key_counter: AtomicU64::new(0),
                task_completed: Notify::new(),
            }),
        }
    }

    /// Create a new OffloadManager with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(OffloadConfig::default())
    }

    /// Generate next auto-incrementing id.
    fn next_id(&self) -> u64 {
        self.inner.key_counter.fetch_add(1, Ordering::Relaxed)
    }

    /// Register a task with the given key.
    ///
    /// If deduplication is enabled and a task with the same `Keyed` key is
    /// already in flight, the new task will be skipped.
    ///
    /// Returns `true` if the task was spawned, `false` if it was deduplicated.
    ///
    /// # Example
    /// ```ignore
    /// use hitbox_core::OffloadKey;
    ///
    /// // With potential deduplication (Keyed)
    /// manager.register((cache_key, "revalidate"), async { /* ... */ });
    ///
    /// // Without deduplication (Explicit id)
    /// manager.register(OffloadKey::explicit("cleanup", 42), async { /* ... */ });
    ///
    /// // Auto-assigned id
    /// manager.register(OffloadKey::auto("background"), async { /* ... */ });
    /// ```
    pub fn register<K, F>(&self, key: K, task: F) -> bool
    where
        K: Into<CoreOffloadKey>,
        F: Future<Output = ()> + Send + 'static,
    {
        // Convert Auto keys to Explicit with auto-assigned id
        let key = match key.into() {
            CoreOffloadKey::Auto { kind } => CoreOffloadKey::Explicit {
                kind,
                id: self.next_id(),
            },
            other => other,
        };

        // Check for deduplication (only for Keyed keys when enabled)
        if self.inner.config.deduplicate
            && matches!(&key, CoreOffloadKey::Keyed { .. })
            && self.inner.tasks.contains_key(&key)
        {
            debug!(?key, "Task deduplicated - already in flight");
            #[cfg(feature = "metrics")]
            metrics::counter!(*OFFLOAD_TASKS_DEDUPLICATED, "kind" => key.kind().to_string())
                .increment(1);
            return false;
        }

        #[cfg(feature = "metrics")]
        let key_kind = key.kind().clone();

        let handle = self.spawn_inner(task, key.clone());
        self.inner.tasks.insert(key, handle);

        #[cfg(feature = "metrics")]
        {
            metrics::counter!(*OFFLOAD_TASKS_SPAWNED, "kind" => key_kind.to_string()).increment(1);
            metrics::gauge!(*OFFLOAD_TASKS_ACTIVE, "kind" => key_kind.to_string()).increment(1.0);
        }

        true
    }

    /// Spawn a task with auto-generated key and specified kind.
    ///
    /// The kind is used for metrics labels and tracing.
    ///
    /// # Deprecation
    ///
    /// This method will be removed in version 0.3. Use [`register`](Self::register) instead.
    ///
    /// # Example
    /// ```ignore
    /// manager.spawn("revalidate", async { /* ... */ });
    /// manager.spawn("warmup", async { /* ... */ });
    /// ```
    #[deprecated(
        since = "0.2.1",
        note = "use `register` instead, will be removed in 0.3"
    )]
    pub fn spawn<F>(&self, kind: impl Into<SmolStr>, task: F) -> CoreOffloadKey
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let kind = kind.into();
        let id = self.next_id();
        let key = CoreOffloadKey::explicit(kind, id);
        self.register(key.clone(), task);
        key
    }

    /// Spawn a task with a specific cache key.
    ///
    /// # Deprecation
    ///
    /// This method will be removed in version 0.3. Use [`register`](Self::register) instead:
    ///
    /// ```ignore
    /// // Before
    /// manager.spawn_with_key(cache_key, "revalidate", task);
    ///
    /// // After
    /// manager.register((cache_key, "revalidate"), task);
    /// ```
    #[deprecated(
        since = "0.2.1",
        note = "use `register` instead, will be removed in 0.3"
    )]
    pub fn spawn_with_key<F>(&self, key: CacheKey, kind: impl Into<SmolStr>, task: F) -> bool
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.register((key, kind), task)
    }

    /// Get the number of currently active tasks.
    pub fn active_task_count(&self) -> usize {
        self.inner.tasks.iter().filter(|e| !e.is_finished()).count()
    }

    /// Get the total number of tracked tasks (including finished).
    pub fn total_task_count(&self) -> usize {
        self.inner.tasks.len()
    }

    /// Clean up finished task handles.
    pub fn cleanup_finished(&self) {
        self.inner.tasks.retain(|_, handle| !handle.is_finished());
    }

    /// Cancel all running tasks.
    pub fn cancel_all(&self) {
        for entry in self.inner.tasks.iter() {
            entry.abort();
        }
    }

    /// Cancel a specific task by key.
    pub fn cancel(&self, key: &CoreOffloadKey) -> bool {
        if let Some(entry) = self.inner.tasks.get(key) {
            entry.abort();
            true
        } else {
            false
        }
    }

    /// Check if a task with the given key is in flight.
    pub fn is_in_flight(&self, key: &CoreOffloadKey) -> bool {
        self.inner.tasks.get(key).is_some_and(|h| !h.is_finished())
    }

    /// Wait for all currently tracked tasks to complete.
    pub async fn wait_all(&self) {
        loop {
            self.cleanup_finished();
            if self.inner.tasks.is_empty() {
                break;
            }
            self.inner.task_completed.notified().await;
        }
    }

    /// Wait for all tasks with a timeout.
    ///
    /// Returns `true` if all tasks completed within the timeout,
    /// `false` if the timeout was reached.
    pub async fn wait_all_timeout(&self, timeout: std::time::Duration) -> bool {
        match tokio::time::timeout(timeout, self.wait_all()).await {
            Ok(()) => true,
            Err(_) => false,
        }
    }

    fn spawn_inner<F>(&self, task: F, key: CoreOffloadKey) -> OffloadHandle
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let timeout_policy = self.inner.config.timeout_policy.clone();
        let inner = self.inner.clone();
        let key_kind = key.kind().clone();

        let span = info_span!(
            "offload_task",
            kind = %key_kind,
            key = ?key,
        );

        let handle = match timeout_policy {
            TimeoutPolicy::None => tokio::spawn(
                async move {
                    #[cfg(feature = "metrics")]
                    let start = Instant::now();
                    task.await;
                    inner.tasks.remove(&key);
                    inner.task_completed.notify_waiters();
                    #[cfg(feature = "metrics")]
                    Self::record_completion(start, &key_kind);
                }
                .instrument(span),
            ),
            TimeoutPolicy::Cancel(duration) => tokio::spawn(
                async move {
                    #[cfg(feature = "metrics")]
                    let start = Instant::now();
                    match tokio::time::timeout(duration, task).await {
                        Ok(()) => {
                            #[cfg(feature = "metrics")]
                            Self::record_completion(start, &key_kind);
                        }
                        Err(_) => {
                            warn!(?key, "Offload task cancelled due to timeout");
                            #[cfg(feature = "metrics")]
                            Self::record_timeout(start, &key_kind);
                        }
                    }
                    inner.tasks.remove(&key);
                    inner.task_completed.notify_waiters();
                }
                .instrument(span),
            ),
            TimeoutPolicy::Warn(duration) => tokio::spawn(
                async move {
                    let start = Instant::now();
                    task.await;
                    let elapsed = start.elapsed();
                    if elapsed > duration {
                        warn!(
                            ?key,
                            elapsed_ms = elapsed.as_millis(),
                            threshold_ms = duration.as_millis(),
                            "Offload task exceeded timeout threshold"
                        );
                    }
                    inner.tasks.remove(&key);
                    inner.task_completed.notify_waiters();
                    #[cfg(feature = "metrics")]
                    Self::record_completion(start, &key_kind);
                }
                .instrument(span),
            ),
        };

        OffloadHandle { handle }
    }

    #[cfg(feature = "metrics")]
    fn record_completion(start: Instant, key_kind: &SmolStr) {
        let duration = start.elapsed().as_secs_f64();
        metrics::counter!(*OFFLOAD_TASKS_COMPLETED, "kind" => key_kind.to_string()).increment(1);
        metrics::gauge!(*OFFLOAD_TASKS_ACTIVE, "kind" => key_kind.to_string()).decrement(1.0);
        metrics::histogram!(*OFFLOAD_TASK_DURATION, "kind" => key_kind.to_string())
            .record(duration);
    }

    #[cfg(feature = "metrics")]
    fn record_timeout(start: Instant, key_kind: &SmolStr) {
        let duration = start.elapsed().as_secs_f64();
        metrics::counter!(*OFFLOAD_TASKS_TIMEOUT, "kind" => key_kind.to_string()).increment(1);
        metrics::gauge!(*OFFLOAD_TASKS_ACTIVE, "kind" => key_kind.to_string()).decrement(1.0);
        metrics::histogram!(*OFFLOAD_TASK_DURATION, "kind" => key_kind.to_string())
            .record(duration);
    }
}

impl Default for OffloadManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl hitbox_core::Offload<'static> for OffloadManager {
    #[allow(deprecated)]
    fn spawn<F>(&self, kind: impl Into<SmolStr>, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        OffloadManager::spawn(self, kind, future);
    }

    fn register<K, F>(&self, key: K, future: F)
    where
        K: Into<CoreOffloadKey>,
        F: Future<Output = ()> + Send + 'static,
    {
        OffloadManager::register(self, key, future);
    }
}
