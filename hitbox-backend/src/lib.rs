#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod backend;
pub mod composition;
pub mod compressor;
pub mod context;
pub mod envelope;
pub mod error;
pub mod format;
pub mod key;
pub(crate) mod metrics;

pub use backend::{Backend, BackendResult, CacheBackend, DeleteStatus, SyncBackend, UnsyncBackend};
pub use composition::{Compose, CompositionBackend};
#[cfg(feature = "gzip")]
#[cfg_attr(docsrs, doc(cfg(feature = "gzip")))]
pub use compressor::GzipCompressor;
#[cfg(feature = "zstd")]
#[cfg_attr(docsrs, doc(cfg(feature = "zstd")))]
pub use compressor::ZstdCompressor;
pub use compressor::{CompressionError, Compressor, PassthroughCompressor};
pub use envelope::ValueEnvelope;
pub use error::BackendError;
#[cfg(feature = "rkyv_format")]
#[cfg_attr(docsrs, doc(cfg(feature = "rkyv_format")))]
pub use format::RkyvFormat;
pub use key::CacheKeyFormat;
