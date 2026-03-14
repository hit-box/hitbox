//! Context types for composition backend operations.
//!
//! This module provides context types that enable refill operations in composition
//! backends when using dynamic dispatch (`Box<dyn Backend>`).

use std::any::Any;

use hitbox_core::{
    BoxContext, CacheContext, CacheStatus, CacheTiming, Context, ReadMode, ResponseSource,
};
use smallbox::smallbox;

use super::CompositionFormat;

/// Source marker indicating which layer provided the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionLayer {
    /// Data came from L1 cache
    L1,
    /// Data came from L2 cache
    L2,
}

/// Context for composition backend operations.
///
/// This context wraps an inner context and adds composition-specific data:
/// - Which layer provided the data (L1/L2)
/// - The composition format for serialization during refill operations
///
/// When data is read from L2 in a composition backend, the context is created
/// with `ReadMode::Refill`, signaling that `CacheBackend::get()` should
/// write the value back to L1 (refill operation).
pub struct CompositionContext {
    /// Inner context for operation tracking (delegates status/source)
    inner: BoxContext,
    /// Which layer the data came from (L1/L2)
    pub layer: CompositionLayer,
    /// The composition format for serialization
    pub format: CompositionFormat,
}

impl CompositionContext {
    /// Wrap an existing context with composition-specific data.
    ///
    /// If the layer is L2, read mode is set to `Refill` to signal that
    /// the data should be written back to L1 for cache refill.
    pub fn wrap(inner: BoxContext, layer: CompositionLayer, format: CompositionFormat) -> Self {
        Self {
            inner,
            layer,
            format,
        }
    }

    /// Returns whether this context should trigger a refill (L2 source).
    pub fn should_refill(&self) -> bool {
        matches!(self.layer, CompositionLayer::L2)
    }

    /// Returns a reference to the inner context.
    pub fn inner(&self) -> &BoxContext {
        &self.inner
    }
}

impl Context for CompositionContext {
    fn status(&self) -> CacheStatus {
        self.inner.status()
    }

    fn set_status(&mut self, status: CacheStatus) {
        self.inner.set_status(status);
    }

    fn source(&self) -> &ResponseSource {
        self.inner.source()
    }

    fn set_source(&mut self, source: ResponseSource) {
        self.inner.set_source(source);
    }

    fn read_mode(&self) -> ReadMode {
        if self.should_refill() {
            ReadMode::Refill
        } else {
            ReadMode::Direct
        }
    }

    fn set_read_mode(&mut self, mode: ReadMode) {
        self.inner.set_read_mode(mode);
    }

    fn timing(&self) -> Option<&CacheTiming> {
        self.inner.timing()
    }

    fn set_timing(&mut self, timing: Option<CacheTiming>) {
        self.inner.set_timing(timing);
    }

    fn stored(&self) -> bool {
        self.inner.stored()
    }

    fn set_stored(&mut self, stored: bool) {
        self.inner.set_stored(stored);
    }

    fn extensions(&self) -> Option<&(dyn Any + Send + Sync)> {
        self.inner.extensions()
    }

    fn set_extensions(&mut self, ext: Option<Box<dyn Any + Send + Sync>>) {
        self.inner.set_extensions(ext);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> BoxContext {
        smallbox!(CompositionContext {
            inner: self.inner.clone_box(),
            layer: self.layer,
            format: self.format.clone(),
        })
    }

    fn into_cache_context(self: Box<Self>) -> CacheContext {
        hitbox_core::finalize_context(self.inner)
    }
}

impl std::fmt::Debug for CompositionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositionContext")
            .field("layer", &self.layer)
            .field("read_mode", &self.read_mode())
            .finish()
    }
}

/// Upgrades a context to a CompositionContext by wrapping it.
///
/// This takes ownership of the existing context and wraps it with
/// composition-specific data.
pub fn upgrade_context(ctx: &mut BoxContext, layer: CompositionLayer, format: CompositionFormat) {
    let inner = std::mem::replace(ctx, CacheContext::default().boxed());
    *ctx = smallbox!(CompositionContext::wrap(inner, layer, format));
}
