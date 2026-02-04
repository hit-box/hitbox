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

use smol_str::SmolStr;

use crate::CacheKey;

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
/// use hitbox_core::Offload;
///
/// fn offload_cache_write<'a, O: Offload<'a>>(offload: &O, key: String) {
///     offload.spawn("cache_write", async move {
///         // Perform background cache write
///         println!("Writing to cache: {}", key);
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
    fn spawn<F>(&self, kind: impl Into<SmolStr>, future: F)
    where
        F: Future<Output = ()> + Send + 'a;

    /// Spawn a future with a specific cache key for deduplication.
    ///
    /// Unlike [`spawn`], this method uses the provided cache key to enable
    /// task deduplication. If a task with the same cache key is already in flight,
    /// the new task may be skipped (depending on the implementation's configuration).
    ///
    /// # Arguments
    ///
    /// * `key` - The cache key used for deduplication.
    /// * `kind` - A label categorizing the task type (e.g., "revalidate", "cache_write").
    ///   Used for metrics and tracing.
    /// * `future` - The future to execute in the background. Must be `Send + 'a`.
    fn spawn_with_key<F>(&self, key: CacheKey, kind: impl Into<SmolStr>, future: F)
    where
        F: Future<Output = ()> + Send + 'a;
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
/// use hitbox_core::{Offload, DisabledOffload};
///
/// let offload = DisabledOffload;
///
/// // This works even with non-'static futures
/// let borrowed_data = String::from("hello");
/// let borrowed_ref = &borrowed_data;
/// offload.spawn("test", async move {
///     // Would use borrowed_ref here
///     let _ = borrowed_ref;
/// });
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DisabledOffload;

impl<'a> Offload<'a> for DisabledOffload {
    #[inline]
    fn spawn<F>(&self, _kind: impl Into<SmolStr>, _future: F)
    where
        F: Future<Output = ()> + Send + 'a,
    {
        // Intentionally does nothing.
        // The future is dropped without execution.
    }

    #[inline]
    fn spawn_with_key<F>(&self, _key: CacheKey, _kind: impl Into<SmolStr>, _future: F)
    where
        F: Future<Output = ()> + Send + 'a,
    {
        // Intentionally does nothing.
        // The future is dropped without execution.
    }
}
