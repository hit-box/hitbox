//! Format implementation for CompositionBackend.
//!
//! This module contains `CompositionFormat` which handles multi-layer serialization
//! for the composition backend, packing data for both L1 and L2 layers.

use std::sync::Arc;

use bytes::Bytes;
use hitbox_core::{BoxContext, CacheValue, Raw, ReadMode};
use smol_str::SmolStr;

use super::context::{CompositionContext, CompositionLayer, upgrade_context};
use super::envelope::CompositionEnvelope;
use crate::Compressor;
use crate::context::Context;
use crate::format::{Format, FormatDeserializer, FormatError, FormatSerializer, FormatTypeId};
use crate::metrics::Timer;

/// Format implementation for CompositionBackend that handles multi-layer serialization.
///
/// This format serializes data in both formats and packs them together into a CompositionEnvelope.
/// On deserialization, it unpacks the CompositionEnvelope and deserializes from L1 if available, otherwise L2.
///
/// Each layer can have its own compression: L1 typically uses PassthroughCompressor (fast memory access),
/// while L2 can use GzipCompressor or ZstdCompressor (reduce network bandwidth).
#[derive(Debug, Clone)]
pub struct CompositionFormat {
    l1_format: Arc<dyn Format>,
    l2_format: Arc<dyn Format>,
    l1_compressor: Arc<dyn Compressor>,
    l2_compressor: Arc<dyn Compressor>,
    /// Pre-computed metrics label for L1: "{composition_name}.{l1_name}"
    l1_label: SmolStr,
    /// Pre-computed metrics label for L2: "{composition_name}.{l2_name}"
    l2_label: SmolStr,
}

impl CompositionFormat {
    /// Create a new CompositionFormat with separate formats and compressors for each layer.
    pub fn new(
        l1_format: Arc<dyn Format>,
        l2_format: Arc<dyn Format>,
        l1_compressor: Arc<dyn Compressor>,
        l2_compressor: Arc<dyn Compressor>,
        l1_label: SmolStr,
        l2_label: SmolStr,
    ) -> Self {
        CompositionFormat {
            l1_format,
            l2_format,
            l1_compressor,
            l2_compressor,
            l1_label,
            l2_label,
        }
    }

    /// Update the metrics labels (called when composition name changes).
    pub fn set_labels(&mut self, l1_label: SmolStr, l2_label: SmolStr) {
        self.l1_label = l1_label;
        self.l2_label = l2_label;
    }

    /// Get the label for a specific layer.
    ///
    /// This is useful for getting the source path after deserialization.
    pub fn label_for_layer(&self, layer: CompositionLayer) -> &SmolStr {
        match layer {
            CompositionLayer::L1 => &self.l1_label,
            CompositionLayer::L2 => &self.l2_label,
        }
    }

    /// Check if L1 and L2 formats are the same type.
    /// Returns true if both formats have the same FormatTypeId.
    fn same_format(&self) -> bool {
        self.l1_format.format_type_id() == self.l2_format.format_type_id()
    }

    /// Serialize data for a specific layer.
    ///
    /// Returns compressed bytes ready to write via `Backend::write`.
    /// Metrics are recorded with the appropriate composed label.
    pub fn serialize_layer(
        &self,
        layer: CompositionLayer,
        f: &mut dyn FnMut(&mut FormatSerializer) -> Result<(), FormatError>,
        context: &dyn Context,
    ) -> Result<Bytes, FormatError> {
        let (format, compressor, label): (&dyn Format, &dyn Compressor, &SmolStr) = match layer {
            CompositionLayer::L1 => (&*self.l1_format, &*self.l1_compressor, &self.l1_label),
            CompositionLayer::L2 => (&*self.l2_format, &*self.l2_compressor, &self.l2_label),
        };

        // Serialize
        let serialize_timer = Timer::new();
        let serialized = format.with_serializer(f, context)?;
        crate::metrics::record_serialize(label, serialize_timer.elapsed());

        // Compress
        let compress_timer = Timer::new();
        let compressed = compressor
            .compress(&serialized)
            .map_err(|e| FormatError::Serialize(Box::new(e)))?;
        crate::metrics::record_compress(label, compress_timer.elapsed());

        Ok(Bytes::from(compressed))
    }

    /// Serialize data for both layers and return raw compressed bytes without Envelope.
    ///
    /// This is used by `CacheBackend::set` for static dispatch where we don't need
    /// the Envelope overhead. Each layer's bytes can be written directly via `Backend::write`.
    ///
    /// Returns `(l1_bytes, l2_bytes)` - compressed and ready to write to each layer.
    ///
    /// Metrics are recorded with composed labels (l1_label, l2_label).
    /// Same-format optimization is applied (serialize once if formats are equal).
    pub fn serialize_parts(
        &self,
        f: &mut dyn FnMut(&mut FormatSerializer) -> Result<(), FormatError>,
        context: &dyn Context,
    ) -> Result<(Bytes, Bytes), FormatError> {
        // Serialize and compress for L1
        let l1_serialize_timer = Timer::new();
        let l1_serialized = self.l1_format.with_serializer(f, context)?;
        crate::metrics::record_serialize(&self.l1_label, l1_serialize_timer.elapsed());

        let l1_compress_timer = Timer::new();
        let l1_compressed = self
            .l1_compressor
            .compress(&l1_serialized)
            .map_err(|e| FormatError::Serialize(Box::new(e)))?;
        crate::metrics::record_compress(&self.l1_label, l1_compress_timer.elapsed());

        // Serialize and compress for L2
        // If L1 and L2 use the same format, reuse the serialized data (skip second serialization)
        let l2_serialized = if self.same_format() {
            l1_serialized.clone()
        } else {
            let l2_serialize_timer = Timer::new();
            let serialized = self.l2_format.with_serializer(f, context)?;
            crate::metrics::record_serialize(&self.l2_label, l2_serialize_timer.elapsed());
            serialized
        };

        let l2_compress_timer = Timer::new();
        let l2_compressed = self
            .l2_compressor
            .compress(&l2_serialized)
            .map_err(|e| FormatError::Serialize(Box::new(e)))?;
        crate::metrics::record_compress(&self.l2_label, l2_compress_timer.elapsed());

        Ok((Bytes::from(l1_compressed), Bytes::from(l2_compressed)))
    }

    /// Deserialize data from a specific layer.
    ///
    /// This is used by `CacheBackend::get` for static dispatch where we read from
    /// a specific layer and need to deserialize without Envelope overhead.
    ///
    /// Metrics are recorded with the appropriate composed label.
    pub fn deserialize_layer(
        &self,
        data: &[u8],
        layer: CompositionLayer,
        f: &mut dyn FnMut(&mut FormatDeserializer) -> Result<(), FormatError>,
        ctx: &mut BoxContext,
    ) -> Result<(), FormatError> {
        let (format, compressor, label): (&dyn Format, &dyn Compressor, &SmolStr) = match layer {
            CompositionLayer::L1 => (&*self.l1_format, &*self.l1_compressor, &self.l1_label),
            CompositionLayer::L2 => (&*self.l2_format, &*self.l2_compressor, &self.l2_label),
        };

        // Decompress
        let decompress_timer = Timer::new();
        let decompressed = compressor
            .decompress(data)
            .map_err(|e| FormatError::Deserialize(Box::new(e)))?;
        crate::metrics::record_decompress(label, decompress_timer.elapsed());

        // Deserialize
        let deserialize_timer = Timer::new();
        format.with_deserializer(&decompressed, f, ctx)?;
        crate::metrics::record_deserialize(label, deserialize_timer.elapsed());

        // NOTE: Don't call upgrade_context here. For nested compositions,
        // the inner CompositionFormat::with_deserializer already upgraded the context.
        // For simple backends, the context remains unchanged, and the caller
        // (CacheBackend::get) will use the backend name directly.
        // The merge_from call in get() adds the composition prefix.

        Ok(())
    }
}

impl Format for CompositionFormat {
    fn with_serializer(
        &self,
        f: &mut dyn FnMut(&mut FormatSerializer) -> Result<(), FormatError>,
        context: &dyn Context,
    ) -> Result<Raw, FormatError> {
        // Check if this is a refill operation (writing L2 data back to L1)
        // CompositionFormat is low-level code that knows about CompositionContext
        if let Some(comp_ctx) = context.as_any().downcast_ref::<CompositionContext>()
            && comp_ctx.read_mode() == ReadMode::Refill
        {
            // For refill operations, create an L1-only envelope
            // This data came from L2, so serialize and compress it for L1 storage
            let serialize_timer = Timer::new();
            let l1_serialized = self.l1_format.with_serializer(f, context)?;
            crate::metrics::record_serialize(&self.l1_label, serialize_timer.elapsed());

            let compress_timer = Timer::new();
            let l1_compressed = self
                .l1_compressor
                .compress(&l1_serialized)
                .map_err(|e| FormatError::Serialize(Box::new(e)))?;
            crate::metrics::record_compress(&self.l1_label, compress_timer.elapsed());

            let composition = CompositionEnvelope::L1(CacheValue::new(
                Bytes::from(l1_compressed),
                None,
                None,
                None,
            ));

            return composition
                .serialize()
                .map_err(|e| FormatError::Serialize(Box::new(e)));
        }

        // Normal write path: Create Both envelope with data for both layers
        // Serialize and compress for L1
        let l1_serialize_timer = Timer::new();
        let l1_serialized = self.l1_format.with_serializer(f, context)?;
        crate::metrics::record_serialize(&self.l1_label, l1_serialize_timer.elapsed());

        let l1_compress_timer = Timer::new();
        let l1_compressed = self
            .l1_compressor
            .compress(&l1_serialized)
            .map_err(|e| FormatError::Serialize(Box::new(e)))?;
        crate::metrics::record_compress(&self.l1_label, l1_compress_timer.elapsed());

        // Serialize and compress for L2
        // If L1 and L2 use the same format, reuse the serialized data instead of serializing again
        let l2_serialized = if self.same_format() {
            l1_serialized.clone()
        } else {
            let l2_serialize_timer = Timer::new();
            let serialized = self.l2_format.with_serializer(f, context)?;
            crate::metrics::record_serialize(&self.l2_label, l2_serialize_timer.elapsed());
            serialized
        };

        let l2_compress_timer = Timer::new();
        let l2_compressed = self
            .l2_compressor
            .compress(&l2_serialized)
            .map_err(|e| FormatError::Serialize(Box::new(e)))?;
        crate::metrics::record_compress(&self.l2_label, l2_compress_timer.elapsed());

        // Pack both compressed values into CompositionEnvelope
        let composition = CompositionEnvelope::Both {
            l1: CacheValue::new(Bytes::from(l1_compressed), None, None, None),
            l2: CacheValue::new(Bytes::from(l2_compressed), None, None, None),
        };

        // Serialize the CompositionEnvelope using zero-copy repr(C) format
        composition
            .serialize()
            .map_err(|e| FormatError::Serialize(Box::new(e)))
    }

    fn with_deserializer(
        &self,
        data: &[u8],
        f: &mut dyn FnMut(&mut FormatDeserializer) -> Result<(), FormatError>,
        ctx: &mut BoxContext,
    ) -> Result<(), FormatError> {
        // Deserialize the CompositionEnvelope using zero-copy repr(C) format
        let composition = CompositionEnvelope::deserialize(data)
            .map_err(|e| FormatError::Deserialize(Box::new(e)))?;

        // Extract source, compressed data, format, compressor, and label from envelope type
        let (compressed_data, format, compressor, source, label): (
            &Bytes,
            &dyn Format,
            &dyn Compressor,
            CompositionLayer,
            &SmolStr,
        ) = match &composition {
            CompositionEnvelope::L1(v) => (
                v.data(),
                &*self.l1_format,
                &*self.l1_compressor,
                CompositionLayer::L1,
                &self.l1_label,
            ),
            CompositionEnvelope::L2(v) => (
                v.data(),
                &*self.l2_format,
                &*self.l2_compressor,
                CompositionLayer::L2,
                &self.l2_label,
            ),
            CompositionEnvelope::Both { l1, .. } => (
                l1.data(),
                &*self.l1_format,
                &*self.l1_compressor,
                CompositionLayer::L1,
                &self.l1_label,
            ),
        };

        // Decompress the data
        let decompress_timer = Timer::new();
        let decompressed = compressor
            .decompress(compressed_data.as_ref())
            .map_err(|e| FormatError::Deserialize(Box::new(e)))?;
        crate::metrics::record_decompress(label, decompress_timer.elapsed());

        // Use the appropriate format to deserialize the decompressed data
        let deserialize_timer = Timer::new();
        format.with_deserializer(&decompressed, f, ctx)?;
        crate::metrics::record_deserialize(label, deserialize_timer.elapsed());

        // Upgrade context to CompositionContext with source layer info
        upgrade_context(ctx, source, self.clone());

        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Format> {
        Box::new(self.clone())
    }

    fn format_type_id(&self) -> FormatTypeId {
        // CompositionFormat is a custom format
        FormatTypeId::Custom("composition")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PassthroughCompressor;
    use crate::format::{BincodeFormat, FormatExt, JsonFormat};
    use hitbox_core::CacheContext;
    use serde::{Deserialize, Serialize};

    #[cfg(feature = "rkyv_format")]
    use rkyv::{Archive, Serialize as RkyvSerialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[cfg_attr(
        feature = "rkyv_format",
        derive(Archive, RkyvSerialize, rkyv::Deserialize)
    )]
    struct TestData {
        id: u32,
        name: String,
        values: Vec<i32>,
    }

    impl TestData {
        fn large() -> Self {
            TestData {
                id: 42,
                name: "test".repeat(100),
                values: (0..1000).collect(),
            }
        }
    }

    #[test]
    fn test_same_format_optimization() {
        let composition = CompositionFormat::new(
            Arc::new(JsonFormat),
            Arc::new(JsonFormat),
            Arc::new(PassthroughCompressor),
            Arc::new(PassthroughCompressor),
            SmolStr::new_static("test.l1"),
            SmolStr::new_static("test.l2"),
        );

        let data = TestData::large();
        let ctx = CacheContext::default();
        let serialized = composition.serialize(&data, &ctx).unwrap();

        let mut boxed_ctx: BoxContext = CacheContext::default().boxed();
        let deserialized: TestData = composition
            .deserialize(&serialized, &mut boxed_ctx)
            .unwrap();
        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_different_formats() {
        let composition = CompositionFormat::new(
            Arc::new(JsonFormat),
            Arc::new(BincodeFormat),
            Arc::new(PassthroughCompressor),
            Arc::new(PassthroughCompressor),
            SmolStr::new_static("test.l1"),
            SmolStr::new_static("test.l2"),
        );

        let data = TestData::large();
        let ctx = CacheContext::default();
        let serialized = composition.serialize(&data, &ctx).unwrap();

        let mut boxed_ctx: BoxContext = CacheContext::default().boxed();
        let deserialized: TestData = composition
            .deserialize(&serialized, &mut boxed_ctx)
            .unwrap();
        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_serialization_size_comparison() {
        let data = TestData::large();
        let ctx = CacheContext::default();

        let json_format = JsonFormat;
        let json_size = json_format.serialize(&data, &ctx).unwrap().len();

        let composition_same = CompositionFormat::new(
            Arc::new(JsonFormat),
            Arc::new(JsonFormat),
            Arc::new(PassthroughCompressor),
            Arc::new(PassthroughCompressor),
            SmolStr::new_static("test.l1"),
            SmolStr::new_static("test.l2"),
        );
        let composition_same_size = composition_same.serialize(&data, &ctx).unwrap().len();

        let composition_diff = CompositionFormat::new(
            Arc::new(JsonFormat),
            Arc::new(BincodeFormat),
            Arc::new(PassthroughCompressor),
            Arc::new(PassthroughCompressor),
            SmolStr::new_static("test.l1"),
            SmolStr::new_static("test.l2"),
        );
        let composition_diff_size = composition_diff.serialize(&data, &ctx).unwrap().len();

        // Composition should be roughly 2x the single format size (plus small bincode overhead)
        assert!(
            composition_same_size < json_size * 2 + 100,
            "Same format composition size should be ~2x single format"
        );

        assert!(
            composition_diff_size > json_size,
            "Different formats should be larger than single JSON"
        );
    }
}
