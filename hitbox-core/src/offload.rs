//! Offload trait for background task execution.
//!
//! This module provides the [`Offload`] trait which abstracts over
//! different implementations for spawning background tasks.
//!
//! # Lifetime Parameter
//!
//! The `Offload<'a>` trait is parameterized by a lifetime to support both:
//! - `'static` futures (for real background execution with `OffloadManager`)
//! - Non-`'static` futures (for middleware integration with `DisabledOffload`)
//!
//! This design allows `CacheFuture` to work with borrowed upstreams (like reqwest
//! middleware's `Next<'_>`) when background revalidation is not needed.

use std::future::Future;
use std::hash::Hash;

use smol_str::SmolStr;

use crate::CacheKey;

/// Key for identifying offloaded tasks.
///
/// This enum represents different types of keys that can be used to identify
/// background tasks.
///
/// # Variants
///
/// - [`Keyed`](OffloadKey::Keyed): Key derived from a cache key.
/// - [`Explicit`](OffloadKey::Explicit): Key with explicit id provided by caller.
/// - [`Auto`](OffloadKey::Auto): Key with auto-assigned id (manager assigns).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OffloadKey {
    /// Key derived from cache key.
    Keyed {
        /// The cache key.
        key: CacheKey,
        /// Kind of the task (e.g., "revalidate", "cache_write").
        kind: SmolStr,
    },
    /// Key with explicit id provided by caller.
    Explicit {
        /// Kind of the task (e.g., "cleanup", "metrics").
        kind: SmolStr,
        /// Unique identifier within the kind.
        id: u64,
    },
    /// Key with auto-assigned id (manager assigns internally).
    Auto {
        /// Kind of the task (e.g., "race_loser", "background").
        kind: SmolStr,
    },
}

impl OffloadKey {
    /// Create a keyed offload key derived from a cache key.
    ///
    /// # Example
    ///
    /// ```
    /// use hitbox_core::{CacheKey, OffloadKey};
    ///
    /// let cache_key = CacheKey::from_str("user", "123");
    /// let key = OffloadKey::keyed(cache_key, "revalidate");
    /// ```
    pub fn keyed(key: CacheKey, kind: impl Into<SmolStr>) -> Self {
        Self::Keyed {
            key,
            kind: kind.into(),
        }
    }

    /// Create a key with explicit id provided by caller.
    ///
    /// # Example
    ///
    /// ```
    /// use hitbox_core::OffloadKey;
    ///
    /// let key = OffloadKey::explicit("cleanup", 42);
    /// ```
    pub fn explicit(kind: impl Into<SmolStr>, id: u64) -> Self {
        Self::Explicit {
            kind: kind.into(),
            id,
        }
    }

    /// Create an auto key where manager assigns id internally.
    ///
    /// # Example
    ///
    /// ```
    /// use hitbox_core::OffloadKey;
    ///
    /// let key = OffloadKey::auto("race_loser");
    /// ```
    pub fn auto(kind: impl Into<SmolStr>) -> Self {
        Self::Auto { kind: kind.into() }
    }

    /// Returns the kind of this key.
    ///
    /// Used for metrics labels and tracing.
    pub fn kind(&self) -> &SmolStr {
        match self {
            Self::Keyed { kind, .. } => kind,
            Self::Explicit { kind, .. } => kind,
            Self::Auto { kind } => kind,
        }
    }
}

/// Conversion from `(CacheKey, S)` tuple to `OffloadKey::Keyed`.
///
/// # Example
///
/// ```
/// use hitbox_core::{CacheKey, OffloadKey};
///
/// let cache_key = CacheKey::from_str("user", "123");
/// let offload_key: OffloadKey = (cache_key, "revalidate").into();
/// ```
impl<S: Into<SmolStr>> From<(CacheKey, S)> for OffloadKey {
    fn from((key, kind): (CacheKey, S)) -> Self {
        Self::keyed(key, kind)
    }
}

/// Trait for spawning background tasks.
///
/// This trait allows components like `CacheFuture` and `CompositionBackend`
/// to offload work to be executed in the background without blocking the main
/// request path.
///
/// # Lifetime Parameter
///
/// The lifetime parameter `'a` determines what futures can be spawned:
/// - `Offload<'static>`: Can spawn futures that live forever (real background tasks)
/// - `Offload<'a>`: Can only spawn futures that live at least as long as `'a`
///
/// This enables [`DisabledOffload`] to accept any lifetime (since it doesn't
/// actually spawn anything), while `OffloadManager` requires `'static`.
///
/// # Implementations
///
/// - [`DisabledOffload`]: Does nothing, accepts any lifetime. Use when background
///   execution is not needed (e.g., reqwest middleware integration).
/// - `OffloadManager` (in `hitbox` crate): Real background execution, requires `'static`.
///
/// # Clone bound
///
/// Implementors should use `Arc` internally to ensure all cloned instances
/// share the same configuration and state.
///
/// # Example
///
/// ```ignore
/// use hitbox_core::{Offload, OffloadKey};
///
/// fn offload_cache_write<'a, O: Offload<'a>>(offload: &O, key: CacheKey) {
///     offload.register((key, "cache_write"), async move {
///         // Perform background cache write
///         println!("Writing to cache");
///     });
/// }
/// ```
pub trait Offload<'a>: Send + Sync + Clone {
    /// Spawn a future to be executed in the background.
    ///
    /// The future will be executed asynchronously and its result will be
    /// handled according to the implementation's policy.
    ///
    /// # Arguments
    ///
    /// * `kind` - A label categorizing the task type (e.g., "revalidate", "cache_write").
    ///   Used for metrics and tracing.
    /// * `future` - The future to execute in the background. Must be `Send + 'a`.
    ///   For real background execution, `'a` must be `'static`.
    ///
    /// # Deprecation
    ///
    /// This method will be removed in version 0.3. Use [`register`](Self::register) instead:
    ///
    /// ```ignore
    /// // Before (deprecated)
    /// offload.spawn("revalidate", async { /* ... */ });
    ///
    /// // After
    /// offload.register(OffloadKey::generated("revalidate", id), async { /* ... */ });
    /// ```
    #[deprecated(
        since = "0.2.1",
        note = "use `register` instead, will be removed in 0.3"
    )]
    fn spawn<F>(&self, kind: impl Into<SmolStr>, future: F)
    where
        F: Future<Output = ()> + Send + 'a;

    /// Register a future to be executed in the background.
    ///
    /// This is the primary method for spawning background tasks.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying this task. A tuple `(CacheKey, &str)` can also be passed.
    /// * `future` - The future to execute in the background. Must be `Send + 'a`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use hitbox_core::{Offload, OffloadKey, CacheKey};
    ///
    /// let cache_key = CacheKey::from_str("user", "123");
    /// offload.register((cache_key, "revalidate"), async {
    ///     // Revalidate cache entry
    /// });
    ///
    /// offload.register(OffloadKey::explicit("cleanup", 1), async {
    ///     // Cleanup task
    /// });
    /// ```
    fn register<K, F>(&self, key: K, future: F)
    where
        K: Into<OffloadKey>,
        F: Future<Output = ()> + Send + 'a,
    {
        let key = key.into();
        #[allow(deprecated)]
        self.spawn(key.kind().clone(), future)
    }
}

/// A disabled offload implementation that discards all spawned tasks.
///
/// This implementation accepts futures with any lifetime since it doesn't
/// actually execute them. Use this when:
/// - Background revalidation is not needed
/// - Integrating with middleware systems that have non-`'static` types
///   (e.g., reqwest middleware's `Next<'_>`)
///
/// # Example
///
/// ```
/// use hitbox_core::{Offload, DisabledOffload, OffloadKey};
///
/// let offload = DisabledOffload;
///
/// // This works even with non-'static futures
/// let borrowed_data = String::from("hello");
/// let borrowed_ref = &borrowed_data;
/// offload.register(OffloadKey::explicit("test", 0), async move {
///     // Would use borrowed_ref here
///     let _ = borrowed_ref;
/// });
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DisabledOffload;

impl<'a> Offload<'a> for DisabledOffload {
    #[inline]
    #[allow(deprecated)]
    fn spawn<F>(&self, _kind: impl Into<SmolStr>, _future: F)
    where
        F: Future<Output = ()> + Send + 'a,
    {
        // Intentionally does nothing.
        // The future is dropped without execution.
    }

    #[inline]
    fn register<K, F>(&self, _key: K, _future: F)
    where
        K: Into<OffloadKey>,
        F: Future<Output = ()> + Send + 'a,
    {
        // Intentionally does nothing.
        // The future is dropped without execution.
    }
}
