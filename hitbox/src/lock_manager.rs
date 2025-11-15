//! Lock manager for preventing dogpile effect in cache misses.
//!
//! This module provides per-key locking using tokio semaphores with LRU eviction
//! to prevent unbounded memory growth. Also provides broadcast channels for sharing
//! responses among waiting requests to avoid cache backend reads.

use std::any::Any;
use std::num::NonZeroUsize;
use std::sync::Arc;
use lru::LruCache;
use parking_lot::Mutex;
use tokio::sync::{broadcast, Semaphore, OwnedSemaphorePermit};
use crate::CacheKey;

/// Error types for lock operations.
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    /// The semaphore was closed (shouldn't normally happen).
    #[error("Semaphore closed")]
    Closed,
}

/// Manages per-key semaphores and broadcast channels with LRU eviction to prevent memory leaks.
///
/// Each cache key gets its own semaphore to control concurrent upstream fetches,
/// and a broadcast channel to share responses among waiting requests without
/// hitting the cache backend.
///
/// # Example
/// ```ignore
/// use hitbox::lock_manager::LockManager;
/// use hitbox::CacheKey;
///
/// let manager = LockManager::new(10_000);
/// let key = CacheKey::from_parts(&[("method", "GET")]);
///
/// // Try to acquire lock
/// if let Some(permit) = manager.try_acquire(&key, 1) {
///     // Got the lock! Fetch from upstream
///     // ...
///     // Broadcast response to waiting requests
///     manager.broadcast_response(&key, Arc::new(response), 1);
/// } else {
///     // Lock held by another request, subscribe to broadcast
///     let receiver = manager.subscribe(&key, 1);
///     let response = receiver.recv().await?;
/// }
/// ```
#[derive(Clone)]
pub struct LockManager {
    semaphores: Arc<Mutex<LruCache<CacheKey, Arc<Semaphore>>>>,
    broadcasts: Arc<Mutex<LruCache<CacheKey, Arc<dyn Any + Send + Sync>>>>,
}

impl LockManager {
    /// Create a new lock manager with the specified LRU capacity.
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of semaphores and broadcast channels to keep in memory.
    ///   When exceeded, least recently used entries are evicted.
    ///
    /// # Panics
    /// Panics if capacity is 0.
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity)
            .expect("Lock manager capacity must be greater than 0");

        Self {
            semaphores: Arc::new(Mutex::new(LruCache::new(capacity))),
            broadcasts: Arc::new(Mutex::new(LruCache::new(capacity))),
        }
    }

    /// Get or create a semaphore for the given cache key.
    ///
    /// This method updates the LRU tracking, moving the semaphore to the
    /// most recently used position.
    fn get_semaphore(&self, key: &CacheKey, concurrency: usize) -> Arc<Semaphore> {
        let mut cache = self.semaphores.lock();
        cache
            .get_or_insert(key.clone(), || Arc::new(Semaphore::new(concurrency)))
            .clone()
    }

    /// Try to acquire a lock immediately without waiting.
    ///
    /// # Arguments
    /// * `key` - The cache key to lock
    /// * `concurrency` - Maximum concurrent fetches allowed for this key
    ///
    /// Returns `Some(permit)` if the lock was acquired, `None` if all permits are taken.
    ///
    /// # Example
    /// ```ignore
    /// if let Some(permit) = manager.try_acquire(&key, 1) {
    ///     // Got it! Fetch upstream
    /// } else {
    ///     // Already locked by another request
    /// }
    /// ```
    pub fn try_acquire(&self, key: &CacheKey, concurrency: usize) -> Option<OwnedSemaphorePermit> {
        let semaphore = self.get_semaphore(key, concurrency);
        semaphore.try_acquire_owned().ok()
    }

    /// Wait to acquire a lock, blocking until it's available.
    ///
    /// This method will wait indefinitely until the lock is released.
    /// The lock is automatically released when the returned permit is dropped.
    ///
    /// # Arguments
    /// * `key` - The cache key to lock
    /// * `concurrency` - Maximum concurrent fetches allowed for this key
    ///
    /// # Errors
    /// Returns `LockError::Closed` if the semaphore was closed (rare).
    ///
    /// # Example
    /// ```ignore
    /// // Wait for lock
    /// let permit = manager.acquire(&key, 1).await?;
    ///
    /// // Check if cache was populated while waiting
    /// if let Some(cached) = backend.get(&key).await? {
    ///     return Ok(cached);
    /// }
    ///
    /// // Still need to fetch
    /// let response = fetch_upstream().await?;
    /// // Permit dropped here, releasing lock
    /// ```
    pub async fn acquire(&self, key: &CacheKey, concurrency: usize) -> Result<OwnedSemaphorePermit, LockError> {
        let semaphore = self.get_semaphore(key, concurrency);
        semaphore
            .acquire_owned()
            .await
            .map_err(|_| LockError::Closed)
    }

    /// Get or create a broadcast channel for the given cache key.
    ///
    /// This method uses type erasure (`Any`) to store broadcast channels for different
    /// response types in the same LRU cache. The type is recovered via downcast when
    /// accessing the channel.
    ///
    /// # Type Safety
    /// The same cache key must always use the same `Cached` type. This is enforced by
    /// the endpoint definition - each endpoint has a specific response type. Using
    /// different types for the same key will cause a panic.
    fn get_or_create_broadcast<Cached>(&self, key: &CacheKey, concurrency: usize) -> broadcast::Sender<Arc<Cached>>
    where
        Cached: Clone + Send + Sync + 'static,
    {
        let mut cache = self.broadcasts.lock();

        let any_sender = cache.get_or_insert(key.clone(), || {
            let (tx, _) = broadcast::channel::<Arc<Cached>>(concurrency.max(1));
            Arc::new(tx) as Arc<dyn Any + Send + Sync>
        }).clone();

        // Downcast back to concrete type
        // This is safe because the same cache key always has the same Cached type
        any_sender
            .downcast_ref::<broadcast::Sender<Arc<Cached>>>()
            .expect("Type mismatch in broadcast channel - same cache key used with different response types")
            .clone()
    }

    /// Subscribe to broadcast channel for the given cache key.
    ///
    /// Returns a receiver that will receive the response when a fetcher broadcasts it.
    /// This allows waiting requests to get responses without reading from the cache backend.
    ///
    /// # Arguments
    /// * `key` - The cache key to subscribe to
    /// * `concurrency` - Buffer size for the broadcast channel (matches max concurrent fetchers)
    ///
    /// # Example
    /// ```ignore
    /// // Waiting request subscribes to broadcast
    /// let mut receiver = manager.subscribe::<SerializableHttpResponse>(&key, 1);
    ///
    /// // Wait for response from fetcher
    /// match receiver.recv().await {
    ///     Ok(cached_response) => {
    ///         // Got response without cache read!
    ///         let response = Response::from_cached(Arc::unwrap_or_clone(cached_response)).await;
    ///     }
    ///     Err(_) => {
    ///         // Channel closed, fallback to cache
    ///     }
    /// }
    /// ```
    pub fn subscribe<Cached>(&self, key: &CacheKey, concurrency: usize) -> broadcast::Receiver<Arc<Cached>>
    where
        Cached: Clone + Send + Sync + 'static,
    {
        let tx = self.get_or_create_broadcast::<Cached>(key, concurrency);
        tx.subscribe()
    }

    /// Broadcast a cached response to all waiting requests.
    ///
    /// This sends the response to all subscribers (waiting requests) for this cache key,
    /// allowing them to receive the response without reading from the cache backend.
    ///
    /// # Arguments
    /// * `key` - The cache key
    /// * `cached` - The cached response wrapped in Arc for cheap cloning
    /// * `concurrency` - Buffer size for the broadcast channel
    ///
    /// # Example
    /// ```ignore
    /// // Fetcher broadcasts response after caching
    /// let cached = SerializableHttpResponse { /* ... */ };
    /// manager.broadcast_response(&key, Arc::new(cached), 1);
    ///
    /// // All waiting requests receive it immediately
    /// ```
    pub fn broadcast_response<Cached>(&self, key: &CacheKey, cached: Arc<Cached>, concurrency: usize)
    where
        Cached: Clone + Send + Sync + 'static,
    {
        let tx = self.get_or_create_broadcast::<Cached>(key, concurrency);
        // Ignore send errors (no receivers is fine - means no one is waiting)
        let _ = tx.send(cached);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_manager_creation() {
        let manager = LockManager::new(100);
        let key = CacheKey::from_slice(&[("test", Some("key"))]);

        // Should be able to acquire with concurrency 1
        assert!(manager.try_acquire(&key, 1).is_some());
    }

    #[test]
    #[should_panic(expected = "capacity must be greater than 0")]
    fn test_zero_capacity_panics() {
        LockManager::new(0);
    }

    #[tokio::test]
    async fn test_lock_prevents_concurrent_access() {
        let manager = LockManager::new(100);
        let key = CacheKey::from_slice(&[("test", Some("key"))]);

        // First acquire should succeed (concurrency 1)
        let permit1 = manager.try_acquire(&key, 1);
        assert!(permit1.is_some());

        // Second acquire should fail (only 1 permit)
        let permit2 = manager.try_acquire(&key, 1);
        assert!(permit2.is_none());

        // Drop first permit
        drop(permit1);

        // Now should succeed
        let permit3 = manager.try_acquire(&key, 1);
        assert!(permit3.is_some());
    }

    #[tokio::test]
    async fn test_acquire_waits_for_release() {
        let manager = LockManager::new(100);
        let key = CacheKey::from_slice(&[("test", Some("key"))]);

        // Acquire lock
        let permit1 = manager.try_acquire(&key, 1).unwrap();

        let manager_clone = manager.clone();
        let key_clone = key.clone();

        // Spawn task that will wait for lock
        let task = tokio::spawn(async move {
            manager_clone.acquire(&key_clone, 1).await
        });

        // Give spawned task time to start waiting
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Release lock
        drop(permit1);

        // Spawned task should now acquire it
        let result = task.await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }

    #[test]
    fn test_lru_eviction() {
        let manager = LockManager::new(2); // Small capacity

        let key1 = CacheKey::from_slice(&[("key", Some("1"))]);
        let key2 = CacheKey::from_slice(&[("key", Some("2"))]);
        let key3 = CacheKey::from_slice(&[("key", Some("3"))]);

        // Acquire and release to add to cache
        drop(manager.try_acquire(&key1, 1));
        drop(manager.try_acquire(&key2, 1));

        // key1 and key2 should be in cache
        // Acquiring key3 should evict key1 (LRU)
        drop(manager.try_acquire(&key3, 1));

        // All should still be acquirable (new semaphores created if evicted)
        assert!(manager.try_acquire(&key1, 1).is_some());
        assert!(manager.try_acquire(&key2, 1).is_some());
        assert!(manager.try_acquire(&key3, 1).is_some());
    }

    #[tokio::test]
    async fn test_concurrency_allows_multiple_acquires() {
        let manager = LockManager::new(100);
        let key = CacheKey::from_slice(&[("test", Some("key"))]);

        // With concurrency 2, should allow 2 concurrent acquires
        let permit1 = manager.try_acquire(&key, 2);
        assert!(permit1.is_some(), "First acquire should succeed");

        let permit2 = manager.try_acquire(&key, 2);
        assert!(permit2.is_some(), "Second acquire should succeed (concurrency=2)");

        // Third should fail
        let permit3 = manager.try_acquire(&key, 2);
        assert!(permit3.is_none(), "Third acquire should fail (only 2 permits)");

        // Drop one permit
        drop(permit1);

        // Now third should succeed
        let permit4 = manager.try_acquire(&key, 2);
        assert!(permit4.is_some(), "Fourth acquire should succeed after release");
    }
}
