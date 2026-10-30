//! Compression strategies for cached values.
//!
//! This module provides compressors that reduce the size of cached data.
//!
//! # Available Compressors
//!
//! | Compressor | Ratio | Speed | Feature |
//! |------------|-------|-------|---------|
//! | [`PassthroughCompressor`] | None | Fastest | - |
//! | `GzipCompressor` | Good | Medium | `gzip` |
//! | `ZstdCompressor` | Best | Fast | `zstd` |

use hitbox_core::Raw;
use thiserror::Error;

/// Error type for compression operations.
#[derive(Debug, Error)]
pub enum CompressionError {
    /// Compression operation failed.
    #[error("Compression failed: {0}")]
    CompressionFailed(String),

    /// Decompression operation failed.
    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),
}

/// Trait for compressing and decompressing cached values.
///
/// Implement this trait to provide custom compression algorithms.
/// Both methods take the buffer by value, so an implementation that leaves
/// the data unchanged can return it without copying.
/// The trait is dyn-compatible with blanket impls for `Box<dyn Compressor>`
/// and `Arc<dyn Compressor>`.
pub trait Compressor: Send + Sync + std::fmt::Debug {
    /// Compress the input data.
    ///
    /// Return a buffer without spare capacity. In-process backends store it
    /// as is, so spare capacity uses memory that their byte limits do not count.
    fn compress(&self, data: Raw) -> Result<Raw, CompressionError>;

    /// Decompress the input data.
    fn decompress(&self, data: Raw) -> Result<Raw, CompressionError>;

    /// Clone this compressor into a box.
    fn clone_box(&self) -> Box<dyn Compressor>;
}

// Blanket implementation for Box<dyn Compressor>
impl Compressor for Box<dyn Compressor> {
    fn compress(&self, data: Raw) -> Result<Raw, CompressionError> {
        (**self).compress(data)
    }

    fn decompress(&self, data: Raw) -> Result<Raw, CompressionError> {
        (**self).decompress(data)
    }

    fn clone_box(&self) -> Box<dyn Compressor> {
        (**self).clone_box()
    }
}

// Blanket implementation for Arc<dyn Compressor>
impl Compressor for std::sync::Arc<dyn Compressor> {
    fn compress(&self, data: Raw) -> Result<Raw, CompressionError> {
        (**self).compress(data)
    }

    fn decompress(&self, data: Raw) -> Result<Raw, CompressionError> {
        (**self).decompress(data)
    }

    fn clone_box(&self) -> Box<dyn Compressor> {
        (**self).clone_box()
    }
}

/// No-op compressor that passes data through unchanged, without copying (default)
#[derive(Debug, Clone, Copy, Default)]
pub struct PassthroughCompressor;

impl Compressor for PassthroughCompressor {
    fn compress(&self, data: Raw) -> Result<Raw, CompressionError> {
        Ok(data)
    }

    fn decompress(&self, data: Raw) -> Result<Raw, CompressionError> {
        Ok(data)
    }

    fn clone_box(&self) -> Box<dyn Compressor> {
        Box::new(*self)
    }
}

/// Gzip compression with configurable level
#[cfg(feature = "gzip")]
#[cfg_attr(docsrs, doc(cfg(feature = "gzip")))]
#[derive(Debug, Clone, Copy)]
pub struct GzipCompressor {
    level: u32,
}

#[cfg(feature = "gzip")]
impl GzipCompressor {
    /// Create a new GzipCompressor with default compression level (6)
    pub fn new() -> Self {
        Self { level: 6 }
    }

    /// Create a new GzipCompressor with specified compression level (0-9)
    pub fn with_level(level: u32) -> Self {
        Self {
            level: level.min(9),
        }
    }
}

#[cfg(feature = "gzip")]
impl Default for GzipCompressor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "gzip")]
impl Compressor for GzipCompressor {
    fn compress(&self, data: Raw) -> Result<Raw, CompressionError> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.level));
        encoder
            .write_all(&data)
            .map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;
        encoder
            .finish()
            .map(|mut buf| {
                buf.shrink_to_fit();
                Raw::from(buf)
            })
            .map_err(|e| CompressionError::CompressionFailed(e.to_string()))
    }

    fn decompress(&self, data: Raw) -> Result<Raw, CompressionError> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(&data[..]);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| CompressionError::DecompressionFailed(e.to_string()))?;
        Ok(Raw::from(decompressed))
    }

    fn clone_box(&self) -> Box<dyn Compressor> {
        Box::new(*self)
    }
}

/// Zstd compression with configurable level
#[cfg(feature = "zstd")]
#[cfg_attr(docsrs, doc(cfg(feature = "zstd")))]
#[derive(Debug, Clone, Copy)]
pub struct ZstdCompressor {
    level: i32,
}

#[cfg(feature = "zstd")]
impl ZstdCompressor {
    /// Create a new ZstdCompressor with default compression level (3)
    pub fn new() -> Self {
        Self { level: 3 }
    }

    /// Create a new ZstdCompressor with specified compression level (-7 to 22)
    /// Lower values = faster but less compression
    /// Higher values = slower but better compression
    pub fn with_level(level: i32) -> Self {
        Self {
            level: level.clamp(-7, 22),
        }
    }
}

#[cfg(feature = "zstd")]
impl Default for ZstdCompressor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "zstd")]
impl Compressor for ZstdCompressor {
    fn compress(&self, data: Raw) -> Result<Raw, CompressionError> {
        zstd::encode_all(&data[..], self.level)
            .map(|mut buf| {
                buf.shrink_to_fit();
                Raw::from(buf)
            })
            .map_err(|e| CompressionError::CompressionFailed(e.to_string()))
    }

    fn decompress(&self, data: Raw) -> Result<Raw, CompressionError> {
        zstd::decode_all(&data[..])
            .map(Raw::from)
            .map_err(|e| CompressionError::DecompressionFailed(e.to_string()))
    }

    fn clone_box(&self) -> Box<dyn Compressor> {
        Box::new(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passthrough_compressor() {
        let compressor = PassthroughCompressor;
        let data = Raw::from(b"Hello, World!".to_vec());

        let compressed = compressor.compress(data.clone()).unwrap();
        assert_eq!(compressed, data);
        // Passthrough hands back the same buffer instead of copying it
        assert_eq!(compressed.as_ptr(), data.as_ptr());

        let decompressed = compressor.decompress(compressed).unwrap();
        assert_eq!(decompressed, data);
        assert_eq!(decompressed.as_ptr(), data.as_ptr());
    }

    /// Capacity of the allocation behind a buffer that no one else references.
    #[cfg(any(feature = "gzip", feature = "zstd"))]
    fn capacity(raw: Raw) -> usize {
        raw.try_into_mut().expect("buffer is not shared").capacity()
    }

    /// Poorly compressible bytes, so the encoder writes its output in several chunks.
    #[cfg(any(feature = "gzip", feature = "zstd"))]
    fn noisy_data() -> Raw {
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        (0..1 << 20)
            .map(|_| {
                // xorshift64
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect::<Vec<u8>>()
            .into()
    }

    #[cfg(feature = "gzip")]
    #[test]
    fn test_gzip_output_has_no_spare_capacity() {
        let compressed = GzipCompressor::new().compress(noisy_data()).unwrap();
        let len = compressed.len();
        assert_eq!(capacity(compressed), len);
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn test_zstd_output_has_no_spare_capacity() {
        let compressed = ZstdCompressor::new().compress(noisy_data()).unwrap();
        let len = compressed.len();
        assert_eq!(capacity(compressed), len);
    }

    #[cfg(feature = "gzip")]
    #[test]
    fn test_gzip_compressor() {
        let compressor = GzipCompressor::new();
        let data = Raw::from(b"Hello, World! This is a test of gzip compression.".repeat(10));

        let compressed = compressor.compress(data.clone()).unwrap();
        assert!(
            compressed.len() < data.len(),
            "Compressed data should be smaller"
        );

        let decompressed = compressor.decompress(compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "gzip")]
    #[test]
    fn test_gzip_compression_levels() {
        let data = Raw::from(b"Hello, World! This is a test of gzip compression.".repeat(100));

        let fast = GzipCompressor::with_level(1);
        let balanced = GzipCompressor::with_level(6);
        let max = GzipCompressor::with_level(9);

        let fast_compressed = fast.compress(data.clone()).unwrap();
        let balanced_compressed = balanced.compress(data.clone()).unwrap();
        let max_compressed = max.compress(data.clone()).unwrap();

        // Higher compression level should produce smaller output
        assert!(max_compressed.len() <= balanced_compressed.len());
        assert!(balanced_compressed.len() <= fast_compressed.len());

        // All should decompress to original
        assert_eq!(fast.decompress(fast_compressed).unwrap(), data);
        assert_eq!(balanced.decompress(balanced_compressed).unwrap(), data);
        assert_eq!(max.decompress(max_compressed).unwrap(), data);
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn test_zstd_compressor() {
        let compressor = ZstdCompressor::new();
        let data = Raw::from(b"Hello, World! This is a test of zstd compression.".repeat(10));

        let compressed = compressor.compress(data.clone()).unwrap();
        assert!(
            compressed.len() < data.len(),
            "Compressed data should be smaller"
        );

        let decompressed = compressor.decompress(compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn test_zstd_compression_levels() {
        let data = Raw::from(b"Hello, World! This is a test of zstd compression.".repeat(100));

        let fast = ZstdCompressor::with_level(-7);
        let balanced = ZstdCompressor::with_level(3);
        let max = ZstdCompressor::with_level(22);

        let fast_compressed = fast.compress(data.clone()).unwrap();
        let balanced_compressed = balanced.compress(data.clone()).unwrap();
        let max_compressed = max.compress(data.clone()).unwrap();

        // Higher compression level should produce smaller output
        assert!(max_compressed.len() <= balanced_compressed.len());

        // All should decompress to original
        assert_eq!(fast.decompress(fast_compressed).unwrap(), data);
        assert_eq!(balanced.decompress(balanced_compressed).unwrap(), data);
        assert_eq!(max.decompress(max_compressed).unwrap(), data);
    }
}
