use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompressionError {
    #[error("Compression failed: {0}")]
    CompressionFailed(String),
    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),
}

/// Trait for compressing and decompressing cached values
pub trait Compressor: Send + Sync + std::fmt::Debug {
    /// Compress the input data
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError>;

    /// Decompress the input data
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError>;
}

// Blanket implementation for Arc<dyn Compressor>
impl Compressor for std::sync::Arc<dyn Compressor> {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        (**self).compress(data)
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        (**self).decompress(data)
    }
}

/// No-op compressor that passes data through unchanged (default)
#[derive(Debug, Clone, Copy, Default)]
pub struct PassthroughCompressor;

impl Compressor for PassthroughCompressor {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        Ok(data.to_vec())
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        Ok(data.to_vec())
    }
}

/// Gzip compression with configurable level
#[cfg(feature = "gzip")]
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
        Self { level: level.min(9) }
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
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.level));
        encoder.write_all(data)
            .map_err(|e| CompressionError::CompressionFailed(e.to_string()))?;
        encoder.finish()
            .map_err(|e| CompressionError::CompressionFailed(e.to_string()))
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        use flate2::read::GzDecoder;
        use std::io::Read;

        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| CompressionError::DecompressionFailed(e.to_string()))?;
        Ok(decompressed)
    }
}

/// Zstd compression with configurable level
#[cfg(feature = "zstd")]
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
        Self { level: level.clamp(-7, 22) }
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
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        zstd::encode_all(data, self.level)
            .map_err(|e| CompressionError::CompressionFailed(e.to_string()))
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        zstd::decode_all(data)
            .map_err(|e| CompressionError::DecompressionFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passthrough_compressor() {
        let compressor = PassthroughCompressor;
        let data = b"Hello, World!";

        let compressed = compressor.compress(data).unwrap();
        assert_eq!(compressed, data);

        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "gzip")]
    #[test]
    fn test_gzip_compressor() {
        let compressor = GzipCompressor::new();
        let data = b"Hello, World! This is a test of gzip compression.".repeat(10);

        let compressed = compressor.compress(&data).unwrap();
        assert!(compressed.len() < data.len(), "Compressed data should be smaller");

        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "gzip")]
    #[test]
    fn test_gzip_compression_levels() {
        let data = b"Hello, World! This is a test of gzip compression.".repeat(100);

        let fast = GzipCompressor::with_level(1);
        let balanced = GzipCompressor::with_level(6);
        let max = GzipCompressor::with_level(9);

        let fast_compressed = fast.compress(&data).unwrap();
        let balanced_compressed = balanced.compress(&data).unwrap();
        let max_compressed = max.compress(&data).unwrap();

        // Higher compression level should produce smaller output
        assert!(max_compressed.len() <= balanced_compressed.len());
        assert!(balanced_compressed.len() <= fast_compressed.len());

        // All should decompress to original
        assert_eq!(fast.decompress(&fast_compressed).unwrap(), data);
        assert_eq!(balanced.decompress(&balanced_compressed).unwrap(), data);
        assert_eq!(max.decompress(&max_compressed).unwrap(), data);
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn test_zstd_compressor() {
        let compressor = ZstdCompressor::new();
        let data = b"Hello, World! This is a test of zstd compression.".repeat(10);

        let compressed = compressor.compress(&data).unwrap();
        assert!(compressed.len() < data.len(), "Compressed data should be smaller");

        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[cfg(feature = "zstd")]
    #[test]
    fn test_zstd_compression_levels() {
        let data = b"Hello, World! This is a test of zstd compression.".repeat(100);

        let fast = ZstdCompressor::with_level(-7);
        let balanced = ZstdCompressor::with_level(3);
        let max = ZstdCompressor::with_level(22);

        let fast_compressed = fast.compress(&data).unwrap();
        let balanced_compressed = balanced.compress(&data).unwrap();
        let max_compressed = max.compress(&data).unwrap();

        // Higher compression level should produce smaller output
        assert!(max_compressed.len() <= balanced_compressed.len());

        // All should decompress to original
        assert_eq!(fast.decompress(&fast_compressed).unwrap(), data);
        assert_eq!(balanced.decompress(&balanced_compressed).unwrap(), data);
        assert_eq!(max.decompress(&max_compressed).unwrap(), data);
    }
}
