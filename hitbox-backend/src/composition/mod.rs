//! Multi-tier caching by combining two backends.
//!
//! This backend implements a layered caching strategy where:
//! - **L1** (first layer): Fast local cache (e.g., Moka)
//! - **L2** (second layer): Distributed cache (e.g., Redis)
//!
//! # Policies
//!
//! Behavior is controlled by configurable policies. See the [`policy`] module for details.
//!
//! ## Read Policies
//! - [`policy::SequentialReadPolicy`] - Try L1, then L2 on miss **(default)**
//! - [`policy::RaceReadPolicy`] - Race both layers, return first hit
//! - [`policy::ParallelReadPolicy`] - Query both in parallel, prefer fresher
//!
//! ## Write Policies
//! - [`policy::SequentialWritePolicy`] - Write L1, then L2
//! - [`policy::OptimisticParallelWritePolicy`] - Write both in parallel **(default)**
//! - [`policy::RaceWritePolicy`] - Race both, background the slower
//!
//! ## Refill Policy
//! - [`policy::RefillPolicy::Always`] - Populate L1 after L2 hit
//! - [`policy::RefillPolicy::Never`] - Skip L1 population **(default)**
//!
//! # Example
//!
//! ```ignore
//! use hitbox_backend::composition::{Compose, CompositionPolicy};
//! use hitbox_backend::composition::policy::{RaceReadPolicy, RefillPolicy};
//!
//! // Default policies
//! let cache = moka.compose(redis, offload);
//!
//! // Custom policies
//! let policy = CompositionPolicy::new()
//!     .read(RaceReadPolicy::new())
//!     .refill(RefillPolicy::Always);
//!
//! let cache = moka.compose(redis, offload).with_policy(policy);
//! ```

pub mod compose;
pub mod policy;

mod context;
mod envelope;
mod format;

pub use compose::Compose;
pub use policy::CompositionPolicy;

// Re-exports for submodules (not part of public API)
pub(crate) use context::{CompositionContext, CompositionLayer};
pub(crate) use format::CompositionFormat;

use crate::format::Format;
use crate::metrics::Timer;
use crate::{
    Backend, BackendError, BackendResult, CacheBackend, CacheKeyFormat, Compressor, DeleteStatus,
    PassthroughCompressor,
};
use async_trait::async_trait;
use envelope::CompositionEnvelope;
use hitbox_core::{
    BackendLabel, BoxContext, CacheContext, CacheKey, CacheStatus, CacheValue, Cacheable,
    CacheableResponse, Offload, Raw, ResponseSource,
};
use policy::{
    CompositionReadPolicy, CompositionWritePolicy, OptimisticParallelWritePolicy, ReadResult,
    RefillPolicy, SequentialReadPolicy,
};
use smol_str::SmolStr;
use std::sync::Arc;
use thiserror::Error;

/// Error type for composition backend operations.
///
/// This error type preserves errors from both cache layers for debugging,
/// while keeping the implementation details encapsulated.
#[derive(Debug, Error)]
pub enum CompositionError {
    /// Both L1 and L2 cache layers failed.
    #[error("Both cache layers failed - L1: {l1}, L2: {l2}")]
    BothLayersFailed {
        /// Error from L1 layer
        l1: BackendError,
        /// Error from L2 layer
        l2: BackendError,
    },
}

/// A backend that composes two cache backends into a layered caching system.
///
/// The first backend (L1) is checked first on reads, and if not found,
/// the second backend (L2) is checked. On writes, both backends are updated.
///
/// Each layer can use its own serialization format and compression since
/// `CacheBackend` operates on typed data, not raw bytes.
///
/// Behavior can be customized via `CompositionReadPolicy`, `CompositionWritePolicy`, and `RefillPolicy` to control
/// how reads, writes, and L1 refills are executed across the layers.
pub struct CompositionBackend<
    L1,
    L2,
    O,
    R = SequentialReadPolicy,
    W = OptimisticParallelWritePolicy,
> where
    L1: Backend,
    L2: Backend,
    O: Offload<'static>,
    R: CompositionReadPolicy,
    W: CompositionWritePolicy,
{
    /// First-layer cache (typically fast, local)
    l1: L1,
    /// Second-layer cache (typically distributed, persistent)
    l2: L2,
    /// Composition format
    format: CompositionFormat,
    /// Offload for background tasks
    offload: O,
    /// Read policy
    read_policy: R,
    /// Write policy
    write_policy: W,
    /// Refill policy
    refill_policy: RefillPolicy,
    /// Label of this backend for source path composition
    label: BackendLabel,
    /// Pre-computed metrics label for L1: "{label}.{l1.label()}"
    l1_label: SmolStr,
    /// Pre-computed metrics label for L2: "{label}.{l2.label()}"
    l2_label: SmolStr,
}

/// Helper to compose a metrics label: "{prefix}.{suffix}"
#[inline]
fn compose_label(prefix: &str, suffix: &str) -> SmolStr {
    SmolStr::from(format!("{}.{}", prefix, suffix))
}

impl<L1, L2, O> CompositionBackend<L1, L2, O, SequentialReadPolicy, OptimisticParallelWritePolicy>
where
    L1: Backend,
    L2: Backend,
    O: Offload<'static>,
{
    /// Creates a new composition backend with two layers using default policies.
    ///
    /// Default policies:
    /// - Read: `SequentialReadPolicy` (try L1 first, then L2)
    /// - Write: `OptimisticParallelWritePolicy` (write to both, succeed if ≥1 succeeds)
    /// - Refill: `RefillPolicy::Never` (do not populate L1 after L2 hit)
    ///
    /// # Arguments
    /// * `l1` - First-layer backend (checked first on reads)
    /// * `l2` - Second-layer backend (checked if L1 misses)
    /// * `offload` - Offload manager for background tasks (e.g., race policy losers)
    pub fn new(l1: L1, l2: L2, offload: O) -> Self {
        let label = BackendLabel::new_static("composition");
        let l1_label = compose_label(label.as_str(), l1.label().as_str());
        let l2_label = compose_label(label.as_str(), l2.label().as_str());
        let format = CompositionFormat::new(
            Arc::new(l1.value_format().clone_box()),
            Arc::new(l2.value_format().clone_box()),
            Arc::new(l1.compressor().clone_box()),
            Arc::new(l2.compressor().clone_box()),
            l1_label.clone(),
            l2_label.clone(),
        );
        Self {
            l1,
            l2,
            format,
            offload,
            read_policy: SequentialReadPolicy::new(),
            write_policy: OptimisticParallelWritePolicy::new(),
            refill_policy: RefillPolicy::default(),
            label,
            l1_label,
            l2_label,
        }
    }
}

impl<L1, L2, O, R, W> CompositionBackend<L1, L2, O, R, W>
where
    L1: Backend,
    L2: Backend,
    O: Offload<'static>,
    R: CompositionReadPolicy,
    W: CompositionWritePolicy,
{
    /// Returns a reference to the read policy.
    pub fn read_policy(&self) -> &R {
        &self.read_policy
    }

    /// Returns a reference to the write policy.
    pub fn write_policy(&self) -> &W {
        &self.write_policy
    }

    /// Returns a reference to the refill policy.
    pub fn refill_policy(&self) -> &RefillPolicy {
        &self.refill_policy
    }

    /// Returns a reference to the offload manager.
    pub fn offload(&self) -> &O {
        &self.offload
    }

    /// Set a custom label for this backend.
    ///
    /// The label is used for source path composition in multi-layer caches.
    /// For example, with label "cache", the source path might be "cache.L1".
    pub fn label(mut self, label: impl Into<BackendLabel>) -> Self {
        self.label = label.into();
        // Recalculate labels with new label
        self.l1_label = compose_label(self.label.as_str(), self.l1.label().as_str());
        self.l2_label = compose_label(self.label.as_str(), self.l2.label().as_str());
        // Update format labels too
        self.format
            .set_labels(self.l1_label.clone(), self.l2_label.clone());
        self
    }

    /// Set all policies at once using CompositionPolicy builder.
    ///
    /// This is the preferred way to configure multiple policies.
    ///
    /// # Example
    /// ```ignore
    /// use hitbox_backend::{CompositionBackend, composition::CompositionPolicy};
    /// use hitbox_backend::composition::policy::{RaceReadPolicy, SequentialWritePolicy, RefillPolicy};
    ///
    /// let policy = CompositionPolicy::new()
    ///     .read(RaceReadPolicy::new())
    ///     .write(SequentialWritePolicy::new())
    ///     .refill(RefillPolicy::Always);
    ///
    /// let backend = CompositionBackend::new(l1, l2, offload)
    ///     .with_policy(policy);
    /// ```
    pub fn with_policy<NewR, NewW>(
        self,
        policy: CompositionPolicy<NewR, NewW>,
    ) -> CompositionBackend<L1, L2, O, NewR, NewW>
    where
        NewR: CompositionReadPolicy,
        NewW: CompositionWritePolicy,
    {
        CompositionBackend {
            l1: self.l1,
            l2: self.l2,
            format: self.format,
            offload: self.offload,
            read_policy: policy.read,
            write_policy: policy.write,
            refill_policy: policy.refill,
            label: self.label,
            l1_label: self.l1_label,
            l2_label: self.l2_label,
        }
    }

    /// Set the read policy (builder pattern).
    ///
    /// This consumes the backend and returns a new one with the updated read policy.
    ///
    /// # Example
    /// ```ignore
    /// use hitbox_backend::CompositionBackend;
    /// use hitbox_backend::composition::policy::RaceReadPolicy;
    ///
    /// let backend = CompositionBackend::new(l1, l2, offload)
    ///     .read(RaceReadPolicy::new());
    /// ```
    pub fn read<NewR: CompositionReadPolicy>(
        self,
        read_policy: NewR,
    ) -> CompositionBackend<L1, L2, O, NewR, W> {
        CompositionBackend {
            l1: self.l1,
            l2: self.l2,
            format: self.format,
            offload: self.offload,
            read_policy,
            write_policy: self.write_policy,
            refill_policy: self.refill_policy,
            label: self.label,
            l1_label: self.l1_label,
            l2_label: self.l2_label,
        }
    }

    /// Set the write policy (builder pattern).
    ///
    /// This consumes the backend and returns a new one with the updated write policy.
    ///
    /// # Example
    /// ```ignore
    /// use hitbox_backend::CompositionBackend;
    /// use hitbox_backend::composition::policy::SequentialWritePolicy;
    ///
    /// let backend = CompositionBackend::new(l1, l2, offload)
    ///     .write(SequentialWritePolicy::new());
    /// ```
    pub fn write<NewW: CompositionWritePolicy>(
        self,
        write_policy: NewW,
    ) -> CompositionBackend<L1, L2, O, R, NewW> {
        CompositionBackend {
            l1: self.l1,
            l2: self.l2,
            format: self.format,
            offload: self.offload,
            read_policy: self.read_policy,
            write_policy,
            refill_policy: self.refill_policy,
            label: self.label,
            l1_label: self.l1_label,
            l2_label: self.l2_label,
        }
    }

    /// Set the refill policy (builder pattern).
    ///
    /// This consumes the backend and returns a new one with the updated refill policy.
    ///
    /// # Example
    /// ```ignore
    /// use hitbox_backend::CompositionBackend;
    /// use hitbox_backend::composition::policy::RefillPolicy;
    ///
    /// let backend = CompositionBackend::new(l1, l2, offload)
    ///     .refill(RefillPolicy::Always);
    /// ```
    pub fn refill(mut self, refill_policy: RefillPolicy) -> Self {
        self.refill_policy = refill_policy;
        self
    }
}

impl<L1, L2, O, R, W> Clone for CompositionBackend<L1, L2, O, R, W>
where
    L1: Clone + Backend,
    L2: Clone + Backend,
    O: Offload<'static>,
    R: Clone + CompositionReadPolicy,
    W: Clone + CompositionWritePolicy,
{
    fn clone(&self) -> Self {
        Self {
            l1: self.l1.clone(),
            l2: self.l2.clone(),
            format: self.format.clone(),
            offload: self.offload.clone(),
            read_policy: self.read_policy.clone(),
            write_policy: self.write_policy.clone(),
            refill_policy: self.refill_policy,
            label: self.label.clone(),
            l1_label: self.l1_label.clone(),
            l2_label: self.l2_label.clone(),
        }
    }
}

impl<L1, L2, O, R, W> std::fmt::Debug for CompositionBackend<L1, L2, O, R, W>
where
    L1: std::fmt::Debug + Backend,
    L2: std::fmt::Debug + Backend,
    O: std::fmt::Debug + Offload<'static>,
    R: std::fmt::Debug + CompositionReadPolicy,
    W: std::fmt::Debug + CompositionWritePolicy,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositionBackend")
            .field("label", &self.label)
            .field("l1", &self.l1)
            .field("l2", &self.l2)
            .field("format", &self.format)
            .field("offload", &self.offload)
            .field("read_policy", &self.read_policy)
            .field("write_policy", &self.write_policy)
            .field("refill_policy", &self.refill_policy)
            .finish()
    }
}

// Backend implementation for CompositionBackend
// This implementation packs/unpacks CompositionEnvelope to enable
// use as Box<dyn Backend> trait object
//
// PERFORMANCE NOTE: Negligible overhead - only metadata (expire/stale timestamps + envelope
// discriminant) is packed into a fixed-size repr(C) header using bytemuck. The already-serialized
// cached data (Bytes) is copied into the buffer as-is without re-serialization. When using
// CompositionBackend directly via CacheBackend::get/set, even this minimal envelope overhead
// is avoided.
#[async_trait]
impl<L1, L2, O, R, W> Backend for CompositionBackend<L1, L2, O, R, W>
where
    L1: Backend + Clone + Send + Sync + 'static,
    L2: Backend + Clone + Send + Sync + 'static,
    O: Offload<'static>,
    R: CompositionReadPolicy,
    W: CompositionWritePolicy,
{
    #[tracing::instrument(skip(self), level = "trace")]
    async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
        // Clone backends for 'static closures
        let l1 = self.l1.clone();
        let l2 = self.l2.clone();
        // Use pre-computed labels (no allocation)
        let l1_label = self.l1_label.clone();
        let l2_label = self.l2_label.clone();

        let read_l1_with_envelope = |k: CacheKey| async move {
            let ctx: BoxContext = CacheContext::default().boxed();
            let timer = Timer::new();
            let read_result = l1.read(&k).await;
            crate::metrics::record_read(&l1_label, timer.elapsed());

            let result = match read_result {
                Ok(Some(l1_value)) => {
                    crate::metrics::record_read_bytes(&l1_label, l1_value.data().len());
                    let (expire, stale) = (l1_value.expire(), l1_value.stale());
                    let envelope = CompositionEnvelope::L1(l1_value);
                    match envelope.serialize() {
                        Ok(packed) => Ok(Some(CacheValue::new(packed, expire, stale, None))),
                        Err(e) => Err(e),
                    }
                }
                Ok(None) => Ok(None),
                Err(e) => {
                    crate::metrics::record_read_error(&l1_label);
                    Err(e)
                }
            };
            (result, ctx)
        };

        let read_l2_with_envelope = |k: CacheKey| async move {
            let ctx: BoxContext = CacheContext::default().boxed();
            let timer = Timer::new();
            let read_result = l2.read(&k).await;
            crate::metrics::record_read(&l2_label, timer.elapsed());

            let result = match read_result {
                Ok(Some(l2_value)) => {
                    crate::metrics::record_read_bytes(&l2_label, l2_value.data().len());
                    let (expire, stale) = (l2_value.expire(), l2_value.stale());
                    let envelope = CompositionEnvelope::L2(l2_value);
                    match envelope.serialize() {
                        Ok(packed) => Ok(Some(CacheValue::new(packed, expire, stale, None))),
                        Err(e) => Err(e),
                    }
                }
                Ok(None) => Ok(None),
                Err(e) => {
                    crate::metrics::record_read_error(&l2_label);
                    Err(e)
                }
            };
            (result, ctx)
        };

        let ReadResult { value, .. } = self
            .read_policy
            .execute_with(
                key.clone(),
                read_l1_with_envelope,
                read_l2_with_envelope,
                &self.offload,
            )
            .await?;

        // No context creation - Format will extract context from envelope during deserialization
        Ok(value)
    }

    #[tracing::instrument(skip(self, value), level = "trace")]
    async fn write(&self, key: &CacheKey, value: CacheValue<Raw>) -> BackendResult<()> {
        // Unpack CompositionEnvelope using zero-copy format
        let composition = CompositionEnvelope::deserialize(value.data())?;

        // Write to appropriate layers
        // In normal usage via CacheBackend::set, this is always Both variant
        // The L1/L2 branches are defensive code for edge cases
        match composition {
            CompositionEnvelope::Both { l1, l2 } => {
                // Clone backends for 'static closures
                let l1_backend = self.l1.clone();
                let l2_backend = self.l2.clone();
                // Use pre-computed labels (no allocation)
                let l1_label = self.l1_label.clone();
                let l2_label = self.l2_label.clone();
                let l1_len = l1.data().len();
                let l2_len = l2.data().len();

                let write_l1 = |k: CacheKey| async move {
                    let timer = Timer::new();
                    let result = l1_backend.write(&k, l1).await;
                    crate::metrics::record_write(&l1_label, timer.elapsed());
                    match &result {
                        Ok(()) => crate::metrics::record_write_bytes(&l1_label, l1_len),
                        Err(_) => crate::metrics::record_write_error(&l1_label),
                    }
                    result
                };
                let write_l2 = |k: CacheKey| async move {
                    let timer = Timer::new();
                    let result = l2_backend.write(&k, l2).await;
                    crate::metrics::record_write(&l2_label, timer.elapsed());
                    match &result {
                        Ok(()) => crate::metrics::record_write_bytes(&l2_label, l2_len),
                        Err(_) => crate::metrics::record_write_error(&l2_label),
                    }
                    result
                };

                self.write_policy
                    .execute_with(key.clone(), write_l1, write_l2, &self.offload)
                    .await
            }
            CompositionEnvelope::L1(l1) => {
                let l1_len = l1.data().len();
                let timer = Timer::new();
                let result = self.l1.write(key, l1).await;
                crate::metrics::record_write(&self.l1_label, timer.elapsed());
                match &result {
                    Ok(()) => crate::metrics::record_write_bytes(&self.l1_label, l1_len),
                    Err(_) => crate::metrics::record_write_error(&self.l1_label),
                }
                result
            }
            CompositionEnvelope::L2(l2) => {
                let l2_len = l2.data().len();
                let timer = Timer::new();
                let result = self.l2.write(key, l2).await;
                crate::metrics::record_write(&self.l2_label, timer.elapsed());
                match &result {
                    Ok(()) => crate::metrics::record_write_bytes(&self.l2_label, l2_len),
                    Err(_) => crate::metrics::record_write_error(&self.l2_label),
                }
                result
            }
        }
    }

    #[tracing::instrument(skip(self), level = "trace")]
    async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
        // Delete from both layers in parallel for better performance
        let (l1_result, l2_result) = futures::join!(self.l1.remove(key), self.l2.remove(key));

        match (l1_result, l2_result) {
            (Err(e1), Err(e2)) => {
                tracing::error!(l1_error = ?e1, l2_error = ?e2, "Both L1 and L2 delete failed");
                Err(BackendError::InternalError(Box::new(
                    CompositionError::BothLayersFailed { l1: e1, l2: e2 },
                )))
            }
            (Err(e), Ok(status)) => {
                tracing::warn!(error = ?e, "L1 delete failed");
                Ok(status)
            }
            (Ok(status), Err(e)) => {
                tracing::warn!(error = ?e, "L2 delete failed");
                Ok(status)
            }
            (Ok(DeleteStatus::Deleted(n1)), Ok(DeleteStatus::Deleted(n2))) => {
                Ok(DeleteStatus::Deleted(n1 + n2))
            }
            (Ok(DeleteStatus::Deleted(n)), Ok(DeleteStatus::Missing))
            | (Ok(DeleteStatus::Missing), Ok(DeleteStatus::Deleted(n))) => {
                Ok(DeleteStatus::Deleted(n))
            }
            (Ok(DeleteStatus::Missing), Ok(DeleteStatus::Missing)) => Ok(DeleteStatus::Missing),
        }
    }

    fn label(&self) -> BackendLabel {
        self.label.clone()
    }

    fn value_format(&self) -> &dyn Format {
        &self.format
    }

    fn key_format(&self) -> &CacheKeyFormat {
        &CacheKeyFormat::Bitcode
    }

    fn compressor(&self) -> &dyn Compressor {
        &PassthroughCompressor
    }
}

impl<L1, L2, O, R, W> CacheBackend for CompositionBackend<L1, L2, O, R, W>
where
    L1: CacheBackend + Clone + Send + Sync + 'static,
    L2: CacheBackend + Clone + Send + Sync + 'static,
    O: Offload<'static>,
    R: CompositionReadPolicy,
    W: CompositionWritePolicy,
{
    #[tracing::instrument(skip(self, ctx), level = "trace")]
    async fn get<T>(
        &self,
        key: &CacheKey,
        ctx: &mut BoxContext,
    ) -> BackendResult<Option<CacheValue<T::Cached>>>
    where
        T: CacheableResponse,
        T::Cached: Cacheable,
    {
        // Clone backends for 'static closures
        let l1 = self.l1.clone();
        let l2 = self.l2.clone();

        // Use pre-computed composed labels for metrics
        let l1_label = self.l1_label.clone();
        let l2_label = self.l2_label.clone();

        // Use inner backend labels for source path (merge_from adds composition prefix)
        let l1_name = l1.label();
        let l2_name = l2.label();

        // Clone format for each closure
        let format_for_l1 = self.format.clone();
        let format_for_l2 = self.format.clone();

        // Clone context for internal L1/L2 operations
        let l1_ctx = ctx.clone_box();
        let l2_ctx = ctx.clone_box();

        let read_l1 = |k: CacheKey| async move {
            let mut internal_ctx = l1_ctx;

            // Read raw bytes from L1 with metrics
            let read_timer = Timer::new();
            let read_result = l1.read(&k).await;
            crate::metrics::record_read(&l1_label, read_timer.elapsed());

            let result = match read_result {
                Ok(Some(raw_value)) => {
                    let (meta, raw_data) = raw_value.into_parts();
                    crate::metrics::record_read_bytes(&l1_label, raw_data.len());

                    // Deserialize using CompositionFormat (records decompress/deserialize metrics)
                    let mut deserialized_opt: Option<T::Cached> = None;
                    match format_for_l1.deserialize_layer(
                        &raw_data,
                        CompositionLayer::L1,
                        &mut |deserializer| {
                            let value: T::Cached = deserializer.deserialize()?;
                            deserialized_opt = Some(value);
                            Ok(())
                        },
                        &mut internal_ctx,
                    ) {
                        Ok(()) => match deserialized_opt {
                            Some(deserialized) => {
                                // Set cache status
                                internal_ctx.set_status(CacheStatus::Hit);

                                // Get source from context (handles nested compositions)
                                // If context was upgraded to CompositionContext, extract source from it
                                let source = if let Some(comp_ctx) =
                                    internal_ctx.as_any().downcast_ref::<CompositionContext>()
                                {
                                    // Nested composition: get label from inner format
                                    BackendLabel::from(
                                        comp_ctx.format.label_for_layer(comp_ctx.layer).clone(),
                                    )
                                } else {
                                    // Simple backend: use backend name
                                    l1_name.clone()
                                };
                                internal_ctx.set_source(ResponseSource::Backend(source));

                                Ok(Some(CacheValue::new(
                                    deserialized,
                                    meta.expire,
                                    meta.stale,
                                    None,
                                )))
                            }
                            None => Err(BackendError::InternalError(Box::new(
                                std::io::Error::other("deserialization produced no result"),
                            ))),
                        },
                        Err(e) => Err(BackendError::InternalError(Box::new(e))),
                    }
                }
                Ok(None) => Ok(None),
                Err(e) => {
                    crate::metrics::record_read_error(&l1_label);
                    Err(e)
                }
            };

            (result, internal_ctx)
        };

        let read_l2 = |k: CacheKey| async move {
            let mut internal_ctx = l2_ctx;

            // Read raw bytes from L2 with metrics
            let read_timer = Timer::new();
            let read_result = l2.read(&k).await;
            crate::metrics::record_read(&l2_label, read_timer.elapsed());

            let result = match read_result {
                Ok(Some(raw_value)) => {
                    let (meta, raw_data) = raw_value.into_parts();
                    crate::metrics::record_read_bytes(&l2_label, raw_data.len());

                    // Deserialize using CompositionFormat (records decompress/deserialize metrics)
                    // Note: deserialize_layer upgrades context to CompositionContext with L2 layer,
                    // which sets ReadMode::Refill - CacheFuture will handle the actual refill
                    let mut deserialized_opt: Option<T::Cached> = None;
                    match format_for_l2.deserialize_layer(
                        &raw_data,
                        CompositionLayer::L2,
                        &mut |deserializer| {
                            let value: T::Cached = deserializer.deserialize()?;
                            deserialized_opt = Some(value);
                            Ok(())
                        },
                        &mut internal_ctx,
                    ) {
                        Ok(()) => match deserialized_opt {
                            Some(deserialized) => {
                                let cache_value =
                                    CacheValue::new(deserialized, meta.expire, meta.stale, None);

                                // Set cache status and source for L2 hit
                                internal_ctx.set_status(CacheStatus::Hit);

                                // Get source from context (handles nested compositions)
                                // If context was upgraded to CompositionContext, extract source from it
                                let source = if let Some(comp_ctx) =
                                    internal_ctx.as_any().downcast_ref::<CompositionContext>()
                                {
                                    // Nested composition: get label from inner format
                                    BackendLabel::from(
                                        comp_ctx.format.label_for_layer(comp_ctx.layer).clone(),
                                    )
                                } else {
                                    // Simple backend: use backend name
                                    l2_name.clone()
                                };
                                internal_ctx.set_source(ResponseSource::Backend(source));

                                Ok(Some(cache_value))
                            }
                            None => Err(BackendError::InternalError(Box::new(
                                std::io::Error::other("deserialization produced no result"),
                            ))),
                        },
                        Err(e) => Err(BackendError::InternalError(Box::new(e))),
                    }
                }
                Ok(None) => Ok(None),
                Err(e) => {
                    crate::metrics::record_read_error(&l2_label);
                    Err(e)
                }
            };

            (result, internal_ctx)
        };

        let ReadResult {
            value,
            source,
            context: inner_ctx,
        } = self
            .read_policy
            .execute_with(key.clone(), read_l1, read_l2, &self.offload)
            .await?;

        // Merge inner context into outer context, composing source paths
        if let Some(ref _cache_value) = value {
            ctx.merge_from(&*inner_ctx, &self.label);

            // If L2 hit and refill policy is Always, set ReadMode::Refill
            // CacheFuture will handle the actual refill via set()
            if source == CompositionLayer::L2 && self.refill_policy == RefillPolicy::Always {
                ctx.set_read_mode(hitbox_core::ReadMode::Refill);
            }
        }

        Ok(value)
    }

    #[tracing::instrument(skip(self, value, ctx), level = "trace")]
    async fn set<T>(
        &self,
        key: &CacheKey,
        value: &CacheValue<T::Cached>,
        ctx: &mut BoxContext,
    ) -> BackendResult<()>
    where
        T: CacheableResponse,
        T::Cached: Cacheable,
    {
        use hitbox_core::ReadMode;

        // Check if this is a refill operation (triggered by CacheFuture after L2 hit)
        // This happens when CacheBackend::get() sets ReadMode::Refill
        if ctx.read_mode() == ReadMode::Refill {
            match self.refill_policy {
                RefillPolicy::Always => {
                    // Refill L1 only - write serialized data to L1
                    let l1_bytes = self
                        .format
                        .serialize_layer(
                            CompositionLayer::L1,
                            &mut |serializer| {
                                serializer.serialize(value.data())?;
                                Ok(())
                            },
                            &**ctx,
                        )
                        .map_err(|e| BackendError::InternalError(Box::new(e)))?;

                    let l1_len = l1_bytes.len();
                    let l1_value = CacheValue::new(l1_bytes, value.expire(), value.stale(), None);

                    // Write to L1 with metrics
                    let timer = Timer::new();
                    let result = self.l1.write(key, l1_value).await;
                    crate::metrics::record_write(&self.l1_label, timer.elapsed());
                    match &result {
                        Ok(()) => crate::metrics::record_write_bytes(&self.l1_label, l1_len),
                        Err(_) => crate::metrics::record_write_error(&self.l1_label),
                    }
                    result?;

                    // Recursively call L2.set() for nested refill
                    // L2 (if it's a CompositionBackend) will handle its own refill logic
                    return self.l2.set::<T>(key, value, ctx).await;
                }
                RefillPolicy::Never => {
                    // With Never policy, don't refill at all
                    // L2 already has the data (it's the source), so skip write
                    return Ok(());
                }
            }
        }

        // Check if this is a nested refill operation via CompositionContext
        // Each CompositionContext wraps an inner context and tracks which layer provided data
        if let Some(comp_ctx) = ctx.as_any().downcast_ref::<CompositionContext>()
            && comp_ctx.layer == CompositionLayer::L2
        {
            match self.refill_policy {
                RefillPolicy::Always => {
                    // This level needs refill: write to L1 only
                    let l1_bytes = self
                        .format
                        .serialize_layer(
                            CompositionLayer::L1,
                            &mut |serializer| {
                                serializer.serialize(value.data())?;
                                Ok(())
                            },
                            &**ctx,
                        )
                        .map_err(|e| BackendError::InternalError(Box::new(e)))?;

                    let l1_len = l1_bytes.len();
                    let l1_value = CacheValue::new(l1_bytes, value.expire(), value.stale(), None);

                    // Write to L1 with metrics
                    let timer = Timer::new();
                    let result = self.l1.write(key, l1_value).await;
                    crate::metrics::record_write(&self.l1_label, timer.elapsed());
                    match &result {
                        Ok(()) => crate::metrics::record_write_bytes(&self.l1_label, l1_len),
                        Err(_) => crate::metrics::record_write_error(&self.l1_label),
                    }
                    result?;

                    // Recursively call L2.set() with inner context for nested refill
                    // Inner context may be another CompositionContext (nested) or CacheContext (leaf)
                    let mut inner_ctx = comp_ctx.inner().clone_box();
                    return self.l2.set::<T>(key, value, &mut inner_ctx).await;
                }
                RefillPolicy::Never => {
                    // Skip L1 write (no refill), but recurse to L2 for nested handling
                    let mut inner_ctx = comp_ctx.inner().clone_box();
                    return self.l2.set::<T>(key, value, &mut inner_ctx).await;
                }
            }
        }

        // Normal mode: write to both layers
        // Serialize for both layers using CompositionFormat
        // This handles same-format optimization and records metrics with composed labels
        let (l1_bytes, l2_bytes) = self
            .format
            .serialize_parts(
                &mut |serializer| {
                    serializer.serialize(value.data())?;
                    Ok(())
                },
                &**ctx,
            )
            .map_err(|e| BackendError::InternalError(Box::new(e)))?;

        let l1_len = l1_bytes.len();
        let l2_len = l2_bytes.len();

        // Create raw values for Backend::write
        let l1_value = CacheValue::new(l1_bytes, value.expire(), value.stale(), None);
        let l2_value = CacheValue::new(l2_bytes, value.expire(), value.stale(), None);

        // Clone backends for 'static closures
        let l1 = self.l1.clone();
        let l2 = self.l2.clone();

        // Use pre-computed composed labels
        let l1_label = self.l1_label.clone();
        let l2_label = self.l2_label.clone();

        // Write closures using Backend::write directly with composed labels
        let write_l1 = |k: CacheKey| async move {
            let timer = Timer::new();
            let result = l1.write(&k, l1_value).await;
            crate::metrics::record_write(&l1_label, timer.elapsed());
            match &result {
                Ok(()) => crate::metrics::record_write_bytes(&l1_label, l1_len),
                Err(_) => crate::metrics::record_write_error(&l1_label),
            }
            result
        };

        let write_l2 = |k: CacheKey| async move {
            let timer = Timer::new();
            let result = l2.write(&k, l2_value).await;
            crate::metrics::record_write(&l2_label, timer.elapsed());
            match &result {
                Ok(()) => crate::metrics::record_write_bytes(&l2_label, l2_len),
                Err(_) => crate::metrics::record_write_error(&l2_label),
            }
            result
        };

        self.write_policy
            .execute_with(key.clone(), write_l1, write_l2, &self.offload)
            .await
    }

    #[tracing::instrument(skip(self, ctx), level = "trace")]
    async fn delete(&self, key: &CacheKey, ctx: &mut BoxContext) -> BackendResult<DeleteStatus> {
        // Delete from both layers in parallel for better performance
        let mut l1_ctx = ctx.clone_box();
        let mut l2_ctx = ctx.clone_box();
        let (l1_result, l2_result) = futures::join!(
            self.l1.delete(key, &mut l1_ctx),
            self.l2.delete(key, &mut l2_ctx)
        );

        // Aggregate results
        match (l1_result, l2_result) {
            (Err(e1), Err(e2)) => {
                tracing::error!(l1_error = ?e1, l2_error = ?e2, "Both L1 and L2 delete failed");
                Err(BackendError::InternalError(Box::new(
                    CompositionError::BothLayersFailed { l1: e1, l2: e2 },
                )))
            }
            (Err(e), Ok(status)) => {
                tracing::warn!(error = ?e, "L1 delete failed");
                Ok(status)
            }
            (Ok(status), Err(e)) => {
                tracing::warn!(error = ?e, "L2 delete failed");
                Ok(status)
            }
            (Ok(DeleteStatus::Deleted(n1)), Ok(DeleteStatus::Deleted(n2))) => {
                tracing::trace!("Deleted from both L1 and L2");
                Ok(DeleteStatus::Deleted(n1 + n2))
            }
            (Ok(DeleteStatus::Deleted(n)), Ok(DeleteStatus::Missing))
            | (Ok(DeleteStatus::Missing), Ok(DeleteStatus::Deleted(n))) => {
                tracing::trace!("Deleted from one layer");
                Ok(DeleteStatus::Deleted(n))
            }
            (Ok(DeleteStatus::Missing), Ok(DeleteStatus::Missing)) => {
                tracing::trace!("Key missing from both layers");
                Ok(DeleteStatus::Missing)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{Format, JsonFormat};
    use crate::{Backend, CacheKeyFormat, Compressor, PassthroughCompressor};
    use async_trait::async_trait;
    use chrono::Utc;
    use hitbox_core::{
        BoxContext, CacheContext, CachePolicy, CacheStatus, CacheValue, CacheableResponse,
        EntityPolicyConfig, Predicate, Raw, ResponseSource,
    };
    use serde::{Deserialize, Serialize};
    use smol_str::SmolStr;
    use std::collections::HashMap;
    use std::future::Future;
    use std::sync::{Arc, Mutex};

    #[cfg(feature = "rkyv_format")]
    use rkyv::{Archive, Serialize as RkyvSerialize};

    /// Test offload that spawns tasks with tokio::spawn
    #[derive(Clone, Debug)]
    struct TestOffload;

    impl Offload<'static> for TestOffload {
        #[allow(deprecated)]
        fn spawn<F>(&self, _kind: impl Into<SmolStr>, future: F)
        where
            F: Future<Output = ()> + Send + 'static,
        {
            tokio::spawn(future);
        }
    }

    // Simple in-memory backend for testing
    #[derive(Clone, Debug)]
    struct TestBackend {
        store: Arc<Mutex<HashMap<CacheKey, CacheValue<Raw>>>>,
        backend_label: &'static str,
    }

    impl TestBackend {
        fn new() -> Self {
            Self {
                store: Arc::new(Mutex::new(HashMap::new())),
                backend_label: "test",
            }
        }

        fn with_label(label: &'static str) -> Self {
            Self {
                store: Arc::new(Mutex::new(HashMap::new())),
                backend_label: label,
            }
        }
    }

    #[async_trait]
    impl Backend for TestBackend {
        async fn read(&self, key: &CacheKey) -> BackendResult<Option<CacheValue<Raw>>> {
            Ok(self.store.lock().unwrap().get(key).cloned())
        }

        async fn write(&self, key: &CacheKey, value: CacheValue<Raw>) -> BackendResult<()> {
            self.store.lock().unwrap().insert(key.clone(), value);
            Ok(())
        }

        async fn remove(&self, key: &CacheKey) -> BackendResult<DeleteStatus> {
            match self.store.lock().unwrap().remove(key) {
                Some(_) => Ok(DeleteStatus::Deleted(1)),
                None => Ok(DeleteStatus::Missing),
            }
        }

        fn label(&self) -> BackendLabel {
            BackendLabel::new(self.backend_label)
        }

        fn value_format(&self) -> &dyn Format {
            &JsonFormat
        }

        fn key_format(&self) -> &CacheKeyFormat {
            &CacheKeyFormat::Bitcode
        }

        fn compressor(&self) -> &dyn Compressor {
            &PassthroughCompressor
        }
    }

    impl CacheBackend for TestBackend {}

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[cfg_attr(
        feature = "rkyv_format",
        derive(Archive, RkyvSerialize, rkyv::Deserialize)
    )]
    struct CachedData {
        value: String,
    }

    // Mock CacheableResponse for testing
    // We only need the associated type, the actual methods are not used in these tests
    struct MockResponse;

    // Note: This is a minimal implementation just for testing CacheBackend.
    // The methods are not actually called in these tests.
    impl CacheableResponse for MockResponse {
        type Cached = CachedData;
        type Subject = MockResponse;
        type IntoCachedFuture = std::future::Ready<CachePolicy<Self::Cached, Self>>;
        type FromCachedFuture = std::future::Ready<Self>;

        async fn cache_policy<P: Predicate<Subject = Self::Subject> + Send + Sync>(
            self,
            _predicate: P,
            _config: &EntityPolicyConfig,
        ) -> CachePolicy<CacheValue<Self::Cached>, Self> {
            unimplemented!("Not used in these tests")
        }

        fn into_cached(self) -> Self::IntoCachedFuture {
            unimplemented!("Not used in these tests")
        }

        fn from_cached(_cached: Self::Cached) -> Self::FromCachedFuture {
            unimplemented!("Not used in these tests")
        }
    }

    #[tokio::test]
    async fn test_l1_hit() {
        let l1 = TestBackend::with_label("moka");
        let l2 = TestBackend::with_label("redis");
        let backend = CompositionBackend::new(l1.clone(), l2, TestOffload).label("cache");

        let key = CacheKey::from_str("test", "key1");
        let value = CacheValue::new(
            CachedData {
                value: "value1".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
            None,
        );

        // Write to populate both layers
        let mut ctx: BoxContext = CacheContext::default().boxed();
        backend
            .set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        // Read should hit L1
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = backend.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(result.unwrap().data().value, "value1");

        // Verify source path is composed correctly: "cache.moka"
        assert_eq!(ctx.status(), CacheStatus::Hit);
        assert_eq!(ctx.source(), &ResponseSource::Backend("cache.moka".into()));
    }

    #[tokio::test]
    async fn test_l2_hit_sets_refill_mode() {
        use hitbox_core::ReadMode;

        let l1 = TestBackend::with_label("moka");
        let l2 = TestBackend::with_label("redis");

        let key = CacheKey::from_str("test", "key1");
        let value = CacheValue::new(
            CachedData {
                value: "value1".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
            None,
        );

        // Backend with RefillPolicy::Always
        let backend = CompositionBackend::new(l1.clone(), l2.clone(), TestOffload)
            .label("cache")
            .refill(RefillPolicy::Always);

        // Write through CompositionBackend (populates both L1 and L2)
        let mut ctx: BoxContext = CacheContext::default().boxed();
        backend
            .set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        // Clear L1 to simulate L1 miss scenario
        l1.store.lock().unwrap().clear();

        // Read should hit L2 and set ReadMode::Refill
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = backend.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(result.unwrap().data().value, "value1");

        // Verify source path is composed correctly: "cache.redis" (hit L2)
        assert_eq!(ctx.status(), CacheStatus::Hit);
        assert_eq!(ctx.source(), &ResponseSource::Backend("cache.redis".into()));

        // Verify ReadMode::Refill is set (CacheFuture will use this to call set())
        assert_eq!(ctx.read_mode(), ReadMode::Refill);

        // L1 should NOT be populated yet (refill happens via CacheFuture.set())
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let l1_result = l1.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert!(
            l1_result.is_none(),
            "L1 should not be populated directly by get()"
        );
    }

    #[tokio::test]
    async fn test_miss_both_layers() {
        let l1 = TestBackend::new();
        let l2 = TestBackend::new();
        let backend = CompositionBackend::new(l1, l2, TestOffload);

        let key = CacheKey::from_str("test", "nonexistent");

        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = backend.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_write_to_both_layers() {
        let l1 = TestBackend::new();
        let l2 = TestBackend::new();

        let key = CacheKey::from_str("test", "key1");
        let value = CacheValue::new(
            CachedData {
                value: "value1".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
            None,
        );

        let backend = CompositionBackend::new(l1.clone(), l2.clone(), TestOffload);

        let mut ctx: BoxContext = CacheContext::default().boxed();
        backend
            .set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        // Verify both layers have the value
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let l1_result = l1.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(l1_result.unwrap().data().value, "value1");

        let mut ctx: BoxContext = CacheContext::default().boxed();
        let l2_result = l2.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(l2_result.unwrap().data().value, "value1");
    }

    #[tokio::test]
    async fn test_delete_from_both_layers() {
        let l1 = TestBackend::new();
        let l2 = TestBackend::new();

        let key = CacheKey::from_str("test", "key1");
        let value = CacheValue::new(
            CachedData {
                value: "value1".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
            None,
        );

        let backend = CompositionBackend::new(l1.clone(), l2.clone(), TestOffload);

        // Write to both
        let mut ctx: BoxContext = CacheContext::default().boxed();
        backend
            .set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        // Delete from both
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let status = backend.delete(&key, &mut ctx).await.unwrap();
        assert_eq!(status, DeleteStatus::Deleted(2));

        // Verify both layers no longer have the value
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let l1_result = l1.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert!(l1_result.is_none());

        let mut ctx: BoxContext = CacheContext::default().boxed();
        let l2_result = l2.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert!(l2_result.is_none());
    }

    #[tokio::test]
    async fn test_clone() {
        let l1 = TestBackend::new();
        let l2 = TestBackend::new();
        let backend = CompositionBackend::new(l1, l2, TestOffload);

        let cloned = backend.clone();

        let key = CacheKey::from_str("test", "key1");
        let value = CacheValue::new(
            CachedData {
                value: "value1".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
            None,
        );

        // Write via original
        let mut ctx: BoxContext = CacheContext::default().boxed();
        backend
            .set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        // Read via clone should work (shared backends)
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = cloned.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(result.unwrap().data().value, "value1");
    }

    #[tokio::test]
    async fn test_nested_composition_source_path() {
        // Create a nested composition: outer(inner(l1, l2), l3)
        // to test hierarchical source paths like "outer.inner.moka"

        let l1 = TestBackend::with_label("moka");
        let l2 = TestBackend::with_label("redis");
        let l3 = TestBackend::with_label("disk");

        // Inner composition: L1=moka, L2=redis
        let inner = CompositionBackend::new(l1.clone(), l2.clone(), TestOffload).label("inner");

        // Outer composition: L1=inner, L2=disk
        let outer = CompositionBackend::new(inner, l3.clone(), TestOffload).label("outer");

        let key = CacheKey::from_str("test", "nested");
        let value = CacheValue::new(
            CachedData {
                value: "nested_value".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
            None,
        );

        // Write only to innermost L1 (moka)
        let mut ctx: BoxContext = CacheContext::default().boxed();
        l1.set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        // Read through outer composition - should hit inner.L1 (moka)
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = outer.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(result.unwrap().data().value, "nested_value");

        // Verify nested source path: "outer.inner.moka"
        assert_eq!(ctx.status(), CacheStatus::Hit);
        assert_eq!(
            ctx.source(),
            &ResponseSource::Backend("outer.inner.moka".into())
        );
    }

    #[tokio::test]
    async fn test_nested_composition_l2_source_path() {
        // Test nested composition where hit comes from inner L2

        let l1 = TestBackend::with_label("moka");
        let l2 = TestBackend::with_label("redis");
        let l3 = TestBackend::with_label("disk");

        // Inner composition: L1=moka, L2=redis
        let inner = CompositionBackend::new(l1.clone(), l2.clone(), TestOffload).label("inner");

        // Outer composition: L1=inner, L2=disk
        let outer = CompositionBackend::new(inner, l3.clone(), TestOffload).label("outer");

        let key = CacheKey::from_str("test", "nested_l2");
        let value = CacheValue::new(
            CachedData {
                value: "from_redis".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
            None,
        );

        // Write only to inner L2 (redis) - not to moka
        let mut ctx: BoxContext = CacheContext::default().boxed();
        l2.set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        // Read through outer composition - should hit inner.L2 (redis)
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = outer.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(result.unwrap().data().value, "from_redis");

        // Verify nested source path: "outer.inner.redis"
        assert_eq!(ctx.status(), CacheStatus::Hit);
        assert_eq!(
            ctx.source(),
            &ResponseSource::Backend("outer.inner.redis".into())
        );
    }

    #[tokio::test]
    async fn test_nested_composition_outer_l2_source_path() {
        // Test nested composition where hit comes from outer L2 (disk)

        let l1 = TestBackend::with_label("moka");
        let l2 = TestBackend::with_label("redis");
        let l3 = TestBackend::with_label("disk");

        // Inner composition: L1=moka, L2=redis
        let inner = CompositionBackend::new(l1.clone(), l2.clone(), TestOffload).label("inner");

        // Outer composition: L1=inner, L2=disk
        let outer = CompositionBackend::new(inner, l3.clone(), TestOffload).label("outer");

        let key = CacheKey::from_str("test", "outer_l2");
        let value = CacheValue::new(
            CachedData {
                value: "from_disk".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
            None,
        );

        // Write only to outer L2 (disk) - not to inner composition
        let mut ctx: BoxContext = CacheContext::default().boxed();
        l3.set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        // Read through outer composition - should hit outer L2 (disk)
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = outer.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(result.unwrap().data().value, "from_disk");

        // Verify source path: "outer.disk"
        assert_eq!(ctx.status(), CacheStatus::Hit);
        assert_eq!(ctx.source(), &ResponseSource::Backend("outer.disk".into()));
    }

    #[tokio::test]
    async fn test_l1_hit_status() {
        let l1 = TestBackend::with_label("moka");
        let l2 = TestBackend::with_label("redis");
        let backend = CompositionBackend::new(l1.clone(), l2, TestOffload).label("cache");

        let key = CacheKey::from_str("test", "metrics1");
        let value = CacheValue::new(
            CachedData {
                value: "value1".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
            None,
        );

        // Write directly to L1 backend to set up the test
        let mut ctx: BoxContext = CacheContext::default().boxed();
        l1.set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        // Read through composition should hit L1
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = backend.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(result.unwrap().data().value, "value1");

        // Verify status and source
        assert_eq!(ctx.status(), CacheStatus::Hit);
        assert_eq!(ctx.source(), &ResponseSource::Backend("cache.moka".into()));
    }

    #[tokio::test]
    async fn test_l2_hit_with_refill_via_set() {
        use hitbox_core::ReadMode;

        let l1 = TestBackend::with_label("moka");
        let l2 = TestBackend::with_label("redis");

        let key = CacheKey::from_str("test", "metrics2");
        let value = CacheValue::new(
            CachedData {
                value: "from_l2".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
            None,
        );

        let backend = CompositionBackend::new(l1.clone(), l2.clone(), TestOffload)
            .label("cache")
            .refill(RefillPolicy::Always);

        // Write through CompositionBackend (populates both L1 and L2)
        let mut ctx: BoxContext = CacheContext::default().boxed();
        backend
            .set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        // Clear L1 to simulate L1 miss scenario
        l1.store.lock().unwrap().clear();

        // Read should hit L2 and set ReadMode::Refill
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = backend.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        let cached_value = result.unwrap();
        assert_eq!(cached_value.data().value, "from_l2");

        // Verify status and source - L2 hit
        assert_eq!(ctx.status(), CacheStatus::Hit);
        assert_eq!(ctx.source(), &ResponseSource::Backend("cache.redis".into()));
        assert_eq!(ctx.read_mode(), ReadMode::Refill);

        // Simulate CacheFuture calling set() with refill context (only writes to L1)
        backend
            .set::<MockResponse>(&key, &cached_value, &mut ctx)
            .await
            .unwrap();

        // Verify L1 was refilled - read again should hit L1
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = backend.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(result.unwrap().data().value, "from_l2");
        assert_eq!(ctx.source(), &ResponseSource::Backend("cache.moka".into()));
    }

    #[tokio::test]
    async fn test_nested_composition_status() {
        let l1 = TestBackend::with_label("moka");
        let l2 = TestBackend::with_label("redis");
        let l3 = TestBackend::with_label("disk");

        let inner = CompositionBackend::new(l1.clone(), l2.clone(), TestOffload).label("inner");
        let outer = CompositionBackend::new(inner, l3.clone(), TestOffload).label("outer");

        let key = CacheKey::from_str("test", "nested_metrics");
        let value = CacheValue::new(
            CachedData {
                value: "nested".to_string(),
            },
            Some(Utc::now() + chrono::Duration::seconds(60)),
            None,
            None,
        );

        // Write to innermost L1 (moka)
        let mut ctx: BoxContext = CacheContext::default().boxed();
        l1.set::<MockResponse>(&key, &value, &mut ctx)
            .await
            .unwrap();

        // Read through outer composition
        let mut ctx: BoxContext = CacheContext::default().boxed();
        let result = outer.get::<MockResponse>(&key, &mut ctx).await.unwrap();
        assert_eq!(result.unwrap().data().value, "nested");

        // Verify nested source path
        assert_eq!(ctx.status(), CacheStatus::Hit);
        assert_eq!(
            ctx.source(),
            &ResponseSource::Backend("outer.inner.moka".into())
        );
    }
}
