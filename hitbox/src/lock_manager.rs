//! Lock manager for preventing dogpile effect in cache misses.
//!
//! This module provides per-key locking using tokio semaphores with LRU eviction
//! to prevent unbounded memory growth.

use std::num::NonZeroUsize;
use std::sync::Arc;
use lru::LruCache;
use parking_lot::Mutex;
use tokio::sync::{Semaphore, OwnedSemaphorePermit};
use crate::CacheKey;

/// Error types for lock operations.
#[derive(Debug, thiserror::Error)]
pub enum LockError {
    /// The semaphore was closed (shouldn't normally happen).
    #[error("Semaphore closed")]
    Closed,
}

/// Manages per-key semaphores with LRU eviction to prevent memory leaks.
///
/// Each cache key gets its own semaphore with 1 permit, ensuring only one
/// request can fetch from upstream for that key at a time.
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
/// if let Some(permit) = manager.try_acquire(&key) {
///     // Got the lock! Fetch from upstream
///     // ...
///     // Permit automatically released on drop
/// } else {
///     // Lock held by another request, wait for it
///     let permit = manager.acquire(&key).await?;
///     // Check if cache was populated while waiting
/// }
/// ```
#[derive(Clone)]
pub struct LockManager {
    semaphores: Arc<Mutex<LruCache<CacheKey, Arc<Semaphore>>>>,
}

impl LockManager {
    /// Create a new lock manager with the specified LRU capacity.
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of semaphores to keep in memory.
    ///   When exceeded, least recently used semaphores are evicted.
    ///
    /// # Panics
    /// Panics if capacity is 0.
    pub fn new(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity)
            .expect("Lock manager capacity must be greater than 0");

        Self {
            semaphores: Arc::new(Mutex::new(LruCache::new(capacity))),
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
