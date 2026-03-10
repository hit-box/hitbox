#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod args;
mod cache;
mod extractor;
mod upstream;

pub use args::{Arg, Args, Skipped};

/// Marker trait for types that can be used as skipped fields in `#[derive(CacheableResponse)]`.
///
/// Skipped fields are not stored in the cache. On cache hit, they are reconstructed
/// using `Default::default()`. This trait provides a clear compile error when a
/// skipped field type doesn't implement `Default`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a skipped field in `#[derive(CacheableResponse)]`",
    note = "skipped fields are not stored in cache and are reconstructed using `Default::default()` on cache hit",
    note = "either implement `Default` for `{Self}` or remove the `#[cacheable_response(skip)]` attribute"
)]
pub trait SkippedFieldDefault {
    /// Create a default value for a skipped field.
    fn skipped_default() -> Self;
}

impl<T: Default> SkippedFieldDefault for T {
    fn skipped_default() -> Self {
        Self::default()
    }
}

/// Marker trait for types that can be used as cached (non-skipped) fields in `#[derive(CacheableResponse)]`.
///
/// Non-skipped fields are stored in the cache and must be cloneable for cache storage.
/// This trait provides a clear compile error when a cached field type doesn't implement `Clone`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a cached field in `#[derive(CacheableResponse)]`",
    note = "non-skipped fields are stored in cache and must implement `Clone` for cache storage",
    note = "either implement `Clone` for `{Self}` or add the `#[cacheable_response(skip)]` attribute"
)]
pub trait CachedFieldClone {
    /// Clone this field for cache storage.
    fn cached_clone(&self) -> Self;
}

impl<T: Clone> CachedFieldClone for T {
    fn cached_clone(&self) -> Self {
        self.clone()
    }
}
pub use cache::{
    Cache, CacheAccess, CacheBuilder, NoBackend, NoContext, NoPolicy, WithBackend, WithContext,
    WithPolicy,
};
pub use extractor::{FnExtractor, KeyExtract};
pub use upstream::FnUpstream;

// Re-export derive macros when feature is enabled
// Note: KeyExtract derive macro shares name with KeyExtract trait (different namespaces)
#[cfg(feature = "derive")]
pub use hitbox_derive::{CacheableRequest, CacheableResponse, KeyExtract, cached};

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::{Arg, Args, Cache, FnExtractor, FnUpstream, KeyExtract, Skipped};

    // Re-export derive macros (KeyExtract derive is re-exported at crate root)
    #[cfg(feature = "derive")]
    pub use hitbox_derive::{CacheableRequest, CacheableResponse, cached};

    // Re-export commonly used hitbox types
    pub use hitbox::KeyPart;
    pub use hitbox::policy::PolicyConfig;
    pub use hitbox::{CacheContext, CacheStatus, ResponseSource};
}
