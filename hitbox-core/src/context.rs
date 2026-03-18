//! Cache context types for tracking cache operation results.

use std::any::Any;

use chrono::{DateTime, Utc};
use smallbox::{SmallBox, smallbox, space::S4};

use crate::label::BackendLabel;

/// Why a request was forwarded to upstream instead of served from cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ForwardReason {
    /// No matching cache entry found.
    #[default]
    Miss,
    /// Cache entry was expired/stale and policy required a fresh response.
    Expired,
    /// Cache was intentionally bypassed (predicate rejected the request).
    /// Covers all predicate rejections including uncacheable methods.
    Bypass,
}

impl ForwardReason {
    /// Returns the reason as a string slice.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            ForwardReason::Miss => "miss",
            ForwardReason::Expired => "expired",
            ForwardReason::Bypass => "bypass",
        }
    }
}

/// What the cache did with this request.
///
/// Variants split into two groups matching RFC 9211 semantics:
/// - Served from cache: `Hit`, `Stale`, `Collapsed` → RFC 9211 `; hit`
/// - Forwarded upstream: `Forward(reason)` → RFC 9211 `; fwd=<reason>`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStatus {
    /// Cache hit — fresh cached data was found and returned.
    Hit,
    /// Stale hit — cached data was served despite being past freshness window
    /// (stale-while-revalidate). Background refresh may be in progress.
    Stale,
    /// Collapsed hit — request was coalesced with another in-flight request
    /// (dog-pile prevention). Served from the other request's result.
    Collapsed,
    /// Forwarded to upstream with a specific reason.
    Forward(ForwardReason),
}

impl Default for CacheStatus {
    fn default() -> Self {
        CacheStatus::Forward(ForwardReason::default())
    }
}

impl CacheStatus {
    /// Returns the status as a string slice.
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            CacheStatus::Hit => "hit",
            CacheStatus::Stale => "stale",
            CacheStatus::Collapsed => "collapsed",
            CacheStatus::Forward(reason) => reason.as_str(),
        }
    }

    /// Returns true if the response was served from cache (hit, stale, or collapsed).
    #[inline]
    pub const fn is_served_from_cache(&self) -> bool {
        matches!(
            self,
            CacheStatus::Hit | CacheStatus::Stale | CacheStatus::Collapsed
        )
    }
}

/// Timing information from a cache operation.
///
/// Used to compute `Age` and `ttl` for cache status headers.
/// Present when response was served from cache (Hit, Stale, Collapsed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheTiming {
    /// When the cache entry was originally created/stored.
    pub created_at: DateTime<Utc>,
    /// When the entry's freshness expires (for ttl computation).
    /// Can be in the past for stale responses → negative ttl.
    pub expire: Option<DateTime<Utc>>,
}

/// Source of the response - either from upstream or from a cache backend.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ResponseSource {
    /// Response came from upstream service (cache miss or bypass).
    #[default]
    Upstream,
    /// Response came from cache backend with the given label.
    Backend(BackendLabel),
}

impl ResponseSource {
    /// Returns the source as a string slice.
    #[inline]
    pub fn as_str(&self) -> &str {
        match self {
            ResponseSource::Upstream => "upstream",
            ResponseSource::Backend(label) => label.as_str(),
        }
    }
}

/// Mode for cache read operations.
///
/// Controls post-read behavior, particularly for composition backends
/// where data read from one layer may need to be written to another.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ReadMode {
    /// Direct read - return value without side effects.
    #[default]
    Direct,
    /// Refill mode - write value back to source layer after reading.
    ///
    /// Used in composition backends to populate L1 with data read from L2.
    Refill,
}

/// Unified context for cache operations.
///
/// This trait combines operation tracking (status, source) with backend policy hints.
/// It allows a single context object to flow through the entire cache pipeline,
/// being transformed as needed by different layers.
///
/// # Usage
///
/// - `CacheFuture` creates a `Box<dyn Context>` at the start
/// - Context is passed as `&mut BoxContext` through backend operations
/// - Backends can upgrade the context type via `*ctx = Box::new(NewContext { ... })`
/// - Format uses `&dyn Context` for policy hints during serialization
/// - At the end, convert to `CacheContext` via `into_cache_context()`
pub trait Context: Send + Sync {
    // Operation tracking

    /// Returns the cache status.
    fn status(&self) -> CacheStatus;

    /// Sets the cache status.
    fn set_status(&mut self, status: CacheStatus);

    /// Returns the response source.
    fn source(&self) -> &ResponseSource;

    /// Sets the response source.
    fn set_source(&mut self, source: ResponseSource);

    // Read mode

    /// Returns the read mode for this context.
    fn read_mode(&self) -> ReadMode {
        ReadMode::default()
    }

    /// Sets the read mode.
    fn set_read_mode(&mut self, _mode: ReadMode) {
        // Default implementation does nothing - simple contexts ignore read mode
    }

    // Cache timing

    /// Returns the cache timing information, if available.
    fn timing(&self) -> Option<&CacheTiming> {
        None
    }

    /// Sets the cache timing information.
    fn set_timing(&mut self, _timing: Option<CacheTiming>) {
        // Default implementation does nothing
    }

    // Stored flag

    /// Returns whether the response was stored in cache during this operation.
    fn stored(&self) -> bool {
        false
    }

    /// Sets whether the response was stored in cache.
    fn set_stored(&mut self, _stored: bool) {
        // Default implementation does nothing
    }

    // Type identity and conversion

    /// Returns a reference to self as `Any` for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Clone this context into a box.
    fn clone_box(&self) -> BoxContext;

    /// Consumes boxed self and returns a `CacheContext`.
    fn into_cache_context(self: Box<Self>) -> CacheContext;

    /// Merge fields from another context into this one.
    ///
    /// Used by composition backends to combine results from inner backends.
    /// The `prefix` is prepended to the source path for hierarchical naming.
    ///
    /// # Arguments
    /// * `other` - The inner context to merge from
    /// * `prefix` - Label prefix to prepend to source path (e.g., backend label)
    fn merge_from(&mut self, other: &dyn Context, prefix: &BackendLabel) {
        // Merge status - take the inner status if it indicates a hit
        let inner_status = other.status();
        if inner_status == CacheStatus::Hit || inner_status == CacheStatus::Stale {
            self.set_status(inner_status);
        }

        // Merge source with path composition
        match other.source() {
            ResponseSource::Backend(inner_label) => {
                // Compose: prefix.inner_label (e.g., "composition.moka")
                let composed = prefix.compose(inner_label);
                self.set_source(ResponseSource::Backend(composed));
            }
            ResponseSource::Upstream => {
                // No backend hit, keep as upstream
            }
        }

        // Merge timing - take from inner if it has timing (cache hit)
        if let Some(timing) = other.timing() {
            self.set_timing(Some(*timing));
        }

        // Merge stored flag
        if other.stored() {
            self.set_stored(true);
        }
    }
}

/// Boxed context trait object using SmallBox for inline storage.
///
/// Uses SmallBox with S4 space (4 * usize = 32 bytes on 64-bit) to avoid
/// heap allocation for small contexts (like `CacheContext`). Larger contexts
/// (like `CompositionContext`) fall back to heap allocation automatically.
///
/// This optimization reduces allocation overhead in the common case
/// where only basic cache context tracking is needed.
pub type BoxContext = SmallBox<dyn Context, S4>;

/// Convert a BoxContext (SmallBox) into a CacheContext.
///
/// This function converts the SmallBox to a Box and then calls
/// `into_cache_context()`. The allocation happens only at the end
/// of the request lifecycle when the context is finalized.
pub fn finalize_context(ctx: BoxContext) -> CacheContext {
    let boxed: Box<dyn Context> = SmallBox::into_box(ctx);
    boxed.into_cache_context()
}

/// Context information about a cache operation.
///
/// This is the single source of truth for all cache operation metadata.
/// The `Ext` type parameter carries protocol-specific data (e.g., HTTP status codes).
///
/// - Inside the FSM: `CacheContext` (= `CacheContext<()>`) — protocol-agnostic
/// - At the integration boundary: `CacheContext<Option<HttpCacheData>>` — protocol-specific
///
/// Use [`with_extensions`](Self::with_extensions) to transform between extension types.
#[derive(Debug, Clone, Default)]
pub struct CacheContext<Ext = ()> {
    /// What the cache did with this request (hit, stale, collapsed, or forwarded).
    pub status: CacheStatus,
    /// Read mode for this operation.
    pub read_mode: ReadMode,
    /// Source of the response.
    pub source: ResponseSource,
    /// Timing data for computing Age and ttl headers.
    /// Present when response was served from cache (Hit, Stale, Collapsed).
    pub timing: Option<CacheTiming>,
    /// Whether the response was stored in cache during this operation.
    pub stored: bool,
    /// Protocol-specific extension data.
    ///
    /// Defaults to `()` inside the FSM. Protocol crates use their own type:
    /// - HTTP: `Option<HttpCacheData>`
    /// - gRPC: `Option<GrpcCacheData>`
    pub extensions: Ext,
}

impl CacheContext {
    /// Convert this context into a boxed trait object.
    ///
    /// This is a convenience method for creating `BoxContext` from `CacheContext`.
    /// Uses SmallBox for inline storage, avoiding heap allocation for small contexts.
    pub fn boxed(self) -> BoxContext {
        smallbox!(self)
    }
}

impl<Ext> CacheContext<Ext> {
    /// Transform this context's extension type.
    ///
    /// Copies all protocol-agnostic fields and replaces extensions with the new value.
    /// Used at the integration boundary to add protocol-specific data.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ctx: CacheContext = finalize_context(box_ctx); // CacheContext<()>
    /// let http_ctx = ctx.with_extensions(Some(HttpCacheData { upstream_status: 200 }));
    /// // http_ctx: CacheContext<Option<HttpCacheData>>
    /// ```
    pub fn with_extensions<NewExt>(self, extensions: NewExt) -> CacheContext<NewExt> {
        CacheContext {
            status: self.status,
            read_mode: self.read_mode,
            source: self.source,
            timing: self.timing,
            stored: self.stored,
            extensions,
        }
    }
}

impl Context for CacheContext {
    fn status(&self) -> CacheStatus {
        self.status
    }

    fn set_status(&mut self, status: CacheStatus) {
        self.status = status;
    }

    fn source(&self) -> &ResponseSource {
        &self.source
    }

    fn set_source(&mut self, source: ResponseSource) {
        self.source = source;
    }

    fn read_mode(&self) -> ReadMode {
        self.read_mode
    }

    fn set_read_mode(&mut self, mode: ReadMode) {
        self.read_mode = mode;
    }

    fn timing(&self) -> Option<&CacheTiming> {
        self.timing.as_ref()
    }

    fn set_timing(&mut self, timing: Option<CacheTiming>) {
        self.timing = timing;
    }

    fn stored(&self) -> bool {
        self.stored
    }

    fn set_stored(&mut self, stored: bool) {
        self.stored = stored;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clone_box(&self) -> BoxContext {
        smallbox!(self.clone())
    }

    fn into_cache_context(self: Box<Self>) -> CacheContext {
        *self
    }
}

/// Extension trait for enriching responses with cache status information.
///
/// This trait provides a protocol-agnostic way to attach cache status
/// metadata to responses. Each protocol (HTTP, gRPC, etc.) implements
/// this trait with its own configuration and extension types.
///
/// # Example
///
/// ```ignore
/// use hitbox_core::{CacheContext, CacheStatusExt};
///
/// // For HTTP responses (implemented in hitbox-http)
/// let http_ctx = cache_context.with_extensions(Some(http_data));
/// response.cache_status(&http_ctx, &config);
/// ```
pub trait CacheStatusExt {
    /// Configuration type for applying cache status (e.g., header name for HTTP).
    type Config;

    /// Protocol-specific extension type carried by [`CacheContext`].
    type Extensions;

    /// Applies cache status information to the response.
    fn cache_status(&mut self, context: &CacheContext<Self::Extensions>, config: &Self::Config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_sizes() {
        use std::mem::size_of;
        let cache_ctx_size = size_of::<CacheContext>();
        let box_ctx_size = size_of::<BoxContext>();
        let s4_space = 4 * size_of::<usize>();

        println!("CacheContext size: {} bytes", cache_ctx_size);
        println!("  - CacheStatus: {} bytes", size_of::<CacheStatus>());
        println!("  - ForwardReason: {} bytes", size_of::<ForwardReason>());
        println!("  - CacheTiming: {} bytes", size_of::<CacheTiming>());
        println!("  - ResponseSource: {} bytes", size_of::<ResponseSource>());
        println!("BoxContext size: {} bytes", box_ctx_size);
        println!("S4 inline space: {} bytes", s4_space);

        // CacheContext now exceeds S4 due to timing + stored fields.
        // SmallBox automatically falls back to heap allocation, which is fine
        // since context is created once per request.
        println!(
            "CacheContext {} S4 inline storage (heap fallback is OK)",
            if cache_ctx_size <= s4_space {
                "fits in"
            } else {
                "exceeds"
            }
        );
    }

    #[test]
    fn test_cache_status_default() {
        let status = CacheStatus::default();
        assert_eq!(status, CacheStatus::Forward(ForwardReason::Miss));
    }

    #[test]
    fn test_cache_status_is_served_from_cache() {
        assert!(CacheStatus::Hit.is_served_from_cache());
        assert!(CacheStatus::Stale.is_served_from_cache());
        assert!(CacheStatus::Collapsed.is_served_from_cache());
        assert!(!CacheStatus::Forward(ForwardReason::Miss).is_served_from_cache());
        assert!(!CacheStatus::Forward(ForwardReason::Expired).is_served_from_cache());
        assert!(!CacheStatus::Forward(ForwardReason::Bypass).is_served_from_cache());
    }

    #[test]
    fn test_with_extensions() {
        let ctx: CacheContext = CacheContext {
            status: CacheStatus::Hit,
            stored: true,
            ..Default::default()
        };
        let ext_ctx = ctx.with_extensions(Some(42u16));
        assert_eq!(ext_ctx.status, CacheStatus::Hit);
        assert!(ext_ctx.stored);
        assert_eq!(ext_ctx.extensions, Some(42u16));
    }
}
