//! Serialization formats for cached values.
//!
//! Cache backends need to serialize values to bytes for storage. The [`Format`] trait
//! provides a dyn-compatible interface that allows backends to select serialization
//! format at runtime.
//!
//! See the [crate-level documentation](crate#serialization-formats) for a comparison
//! of available formats.
//!
//! # Why dyn-compatible?
//!
//! The [`Backend`](crate::Backend) trait returns its format via `value_format() -> &dyn Format`.
//! This design enables:
//!
//! - **Runtime format selection**: Choose format based on configuration, not compile-time generics
//! - **Heterogeneous backends**: Combine backends with different formats in composition layers
//! - **Format switching**: Change serialization strategy without recompiling
//!
//! Making [`Format`] dyn-compatible required a callback-based API ([`Format::with_serializer`],
//! [`Format::with_deserializer`]) instead of returning serializers directly. This avoids
//! self-referential lifetime issues that would prevent trait object usage.
//!
//! # Extending with Custom Formats
//!
//! Implement [`Format`] to add custom serialization. Use [`FormatTypeId::Custom`] with
//! a unique identifier string to ensure your format can be distinguished from built-in ones.

use hitbox_core::{Cacheable, Raw};
use thiserror::Error;

use hitbox_core::BoxContext;

use crate::context::Context;

#[cfg(feature = "rkyv_format")]
use ::rkyv::{api::high::to_bytes_in, from_bytes, rancor, util::AlignedVec};

// Bincode imports for concrete types (use absolute paths to avoid conflict with our bincode module)
use ::bincode::config::Configuration;
use ::bincode::de::DecoderImpl;
use ::bincode::de::read::SliceReader;
use ::bincode::enc::EncoderImpl;
use ::bincode::serde::Compat;
use ::bincode::{Decode, Encode};

// Import the BincodeVecWriter from bincode module
use self::bincode::BincodeVecWriter;

/// Opaque bincode encoder wrapper.
///
/// This type is exposed in [`FormatSerializer::Bincode`] but cannot be
/// constructed outside this crate. Use [`FormatSerializer::Serde`] for
/// custom format implementations.
pub struct BincodeEncoder<'a>(pub(crate) &'a mut EncoderImpl<BincodeVecWriter, Configuration>);

/// Opaque bincode decoder wrapper.
///
/// This type is exposed in [`FormatDeserializer::Bincode`] but cannot be
/// constructed outside this crate. Use [`FormatDeserializer::Serde`] for
/// custom format implementations.
pub struct BincodeDecoder<'a>(pub(crate) &'a mut DecoderImpl<SliceReader<'a>, Configuration, ()>);

mod bincode;
mod json;
#[cfg(feature = "rkyv_format")]
mod rkyv;
mod ron;

pub use bincode::BincodeFormat;
pub use json::JsonFormat;
#[cfg(feature = "rkyv_format")]
#[cfg_attr(docsrs, doc(cfg(feature = "rkyv_format")))]
pub use rkyv::RkyvFormat;
pub use ron::RonFormat;

/// Errors from serialization and deserialization operations.
#[derive(Error, Debug)]
pub enum FormatError {
    /// Serialization failed.
    #[error(transparent)]
    Serialize(Box<dyn std::error::Error + Send>),

    /// Deserialization failed.
    #[error(transparent)]
    Deserialize(Box<dyn std::error::Error + Send>),
}

/// Error type for rkyv serialization that preserves error information
/// when the actual error type cannot be directly boxed due to trait object constraints
#[cfg(feature = "rkyv_format")]
#[derive(Error, Debug)]
#[error("rkyv serialization failed: {message}")]
struct RkyvSerializeError {
    message: String,
}

#[cfg(feature = "rkyv_format")]
impl RkyvSerializeError {
    fn new(error: impl std::fmt::Debug) -> Self {
        Self {
            message: format!("{:?}", error),
        }
    }
}

/// Error type for rkyv validation that preserves error information
/// The underlying CheckArchiveError contains non-Send trait objects, so we
/// capture the error message as a Send-safe string
#[cfg(feature = "rkyv_format")]
#[derive(Error, Debug)]
#[error("rkyv validation failed: {message}")]
struct RkyvValidationError {
    message: String,
}

#[cfg(feature = "rkyv_format")]
impl RkyvValidationError {
    fn new(error: impl std::fmt::Display) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

/// Identifies a format type for equality comparison.
///
/// Since [`Format`] is dyn-compatible, you cannot compare formats with `TypeId`.
/// This enum provides a stable identifier that works across trait objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormatTypeId {
    /// JSON format.
    Json,
    /// Bincode format.
    Bincode,
    /// RON format.
    Ron,
    /// Rkyv format.
    Rkyv,
    /// User-defined format. The string must be globally unique.
    Custom(&'static str),
}

/// Serializer passed to [`Format::with_serializer`] callbacks.
///
/// Wraps different serialization backends (serde, rkyv, bincode) in a unified interface.
/// Call [`serialize`](Self::serialize) with any [`Cacheable`] value.
pub enum FormatSerializer<'a> {
    /// Serde-based serializer (used by JSON, RON).
    Serde(&'a mut dyn erased_serde::Serializer),
    /// Rkyv serializer (zero-copy).
    #[cfg(feature = "rkyv_format")]
    Rkyv(&'a mut AlignedVec),
    /// Bincode serializer (opaque, cannot be constructed externally).
    Bincode(BincodeEncoder<'a>),
}

impl<'a> FormatSerializer<'a> {
    /// Serializes a value using the underlying format.
    ///
    /// Dispatches to the appropriate serialization backend automatically.
    pub fn serialize<T>(&mut self, value: &T) -> Result<(), FormatError>
    where
        T: Cacheable,
    {
        match self {
            FormatSerializer::Serde(ser) => {
                let erased_value = value as &dyn erased_serde::Serialize;
                erased_value
                    .erased_serialize(*ser)
                    .map_err(|e| FormatError::Serialize(Box::new(e)))
            }
            #[cfg(feature = "rkyv_format")]
            FormatSerializer::Rkyv(buffer) => {
                // Serialize directly into the pre-allocated buffer (no double allocation)
                // Take ownership temporarily, serialize, then restore the buffer
                let mut owned_buffer = std::mem::take(*buffer);
                owned_buffer.clear();
                let result_buffer = to_bytes_in::<_, rancor::Error>(value, owned_buffer)
                    .map_err(|e| FormatError::Serialize(Box::new(RkyvSerializeError::new(e))))?;
                **buffer = result_buffer;
                Ok(())
            }
            FormatSerializer::Bincode(enc) => {
                // Use Compat wrapper to bridge serde and bincode
                let compat = Compat(value);
                Encode::encode(&compat, enc.0).map_err(|e| FormatError::Serialize(Box::new(e)))
            }
        }
    }
}

/// Deserializer passed to [`Format::with_deserializer`] callbacks.
///
/// Wraps different deserialization backends in a unified interface.
/// Call [`deserialize`](Self::deserialize) to reconstruct the original value.
pub enum FormatDeserializer<'a> {
    /// Serde-based deserializer (used by JSON, RON).
    Serde(&'a mut dyn erased_serde::Deserializer<'a>),
    /// Rkyv deserializer (validates and deserializes archived bytes).
    #[cfg(feature = "rkyv_format")]
    Rkyv(&'a [u8]),
    /// Bincode deserializer (opaque, cannot be constructed externally).
    Bincode(BincodeDecoder<'a>),
}

impl<'a> FormatDeserializer<'a> {
    /// Deserializes a value using the underlying format.
    ///
    /// Dispatches to the appropriate deserialization backend automatically.
    pub fn deserialize<T>(&mut self) -> Result<T, FormatError>
    where
        T: Cacheable,
    {
        match self {
            FormatDeserializer::Serde(deser) => {
                erased_serde::deserialize(*deser).map_err(|e| FormatError::Deserialize(Box::new(e)))
            }
            #[cfg(feature = "rkyv_format")]
            FormatDeserializer::Rkyv(data) => {
                // Use rkyv 0.8's from_bytes API which validates and deserializes
                let value: T = from_bytes::<T, rancor::Error>(data)
                    .map_err(|e| FormatError::Deserialize(Box::new(RkyvValidationError::new(e))))?;
                Ok(value)
            }
            FormatDeserializer::Bincode(dec) => {
                // Use Compat wrapper to decode from bincode
                let compat: Compat<T> =
                    Decode::decode(dec.0).map_err(|e| FormatError::Deserialize(Box::new(e)))?;
                Ok(compat.0)
            }
        }
    }
}

/// Dyn-compatible serialization format.
///
/// Uses a callback-based API to work around lifetime constraints that would
/// prevent returning serializers directly from trait methods.
///
/// # For Implementors
///
/// Implement [`with_serializer`](Self::with_serializer) and [`with_deserializer`](Self::with_deserializer)
/// to provide serialization/deserialization. The callback receives a [`FormatSerializer`] or
/// [`FormatDeserializer`] that handles the actual conversion.
///
/// # For Callers
///
/// Use [`FormatExt::serialize`] and [`FormatExt::deserialize`] instead of calling
/// the callback methods directly. The extension trait provides a cleaner API.
pub trait Format: std::fmt::Debug + Send + Sync {
    /// Serializes a value through a callback.
    ///
    /// Creates a serializer, passes it to the callback, and returns the serialized bytes.
    ///
    /// Return a buffer without spare capacity. In-process backends store it
    /// as is, so spare capacity uses memory that their byte limits do not count.
    fn with_serializer(
        &self,
        f: &mut dyn FnMut(&mut FormatSerializer) -> Result<(), FormatError>,
        context: &dyn Context,
    ) -> Result<Raw, FormatError>;

    /// Deserializes a value through a callback.
    ///
    /// Creates a deserializer from the data and passes it to the callback.
    /// The context can be modified during deserialization (e.g., to upgrade schema versions).
    fn with_deserializer(
        &self,
        data: &[u8],
        f: &mut dyn FnMut(&mut FormatDeserializer) -> Result<(), FormatError>,
        ctx: &mut BoxContext,
    ) -> Result<(), FormatError>;

    /// Clones this format into a boxed trait object.
    fn clone_box(&self) -> Box<dyn Format>;

    /// Returns this format's type identifier.
    fn format_type_id(&self) -> FormatTypeId;
}

/// Ergonomic serialization methods for [`Format`].
///
/// Provides typed `serialize` and `deserialize` methods. This trait is automatically
/// implemented for all `Format` types via blanket implementation.
pub trait FormatExt: Format {
    /// Serializes a value to raw bytes.
    fn serialize<T>(&self, value: &T, context: &dyn Context) -> Result<Raw, FormatError>
    where
        T: Cacheable,
    {
        self.with_serializer(&mut |serializer| serializer.serialize(value), context)
    }

    /// Deserializes raw bytes into a value.
    fn deserialize<T>(&self, data: &Raw, ctx: &mut BoxContext) -> Result<T, FormatError>
    where
        T: Cacheable,
    {
        let mut result: Option<T> = None;
        self.with_deserializer(
            data,
            &mut |deserializer| {
                let value: T = deserializer.deserialize()?;
                result = Some(value);
                Ok(())
            },
            ctx,
        )?;

        result.ok_or_else(|| {
            FormatError::Deserialize(Box::new(std::io::Error::other(
                "deserialization produced no result",
            )))
        })
    }
}

// Blanket implementation: all Formats automatically get generic methods
impl<T: Format + ?Sized> FormatExt for T {}

// Implement Clone for Box<dyn Format>
impl Clone for Box<dyn Format> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

// Implement Format for Box<dyn Format>
impl Format for Box<dyn Format> {
    fn with_serializer(
        &self,
        f: &mut dyn FnMut(&mut FormatSerializer) -> Result<(), FormatError>,
        context: &dyn Context,
    ) -> Result<Raw, FormatError> {
        (**self).with_serializer(f, context)
    }

    fn with_deserializer(
        &self,
        data: &[u8],
        f: &mut dyn FnMut(&mut FormatDeserializer) -> Result<(), FormatError>,
        ctx: &mut BoxContext,
    ) -> Result<(), FormatError> {
        (**self).with_deserializer(data, f, ctx)
    }

    fn clone_box(&self) -> Box<dyn Format> {
        (**self).clone_box()
    }

    fn format_type_id(&self) -> FormatTypeId {
        (**self).format_type_id()
    }
}

// Implement Format for Arc<dyn Format>
impl Format for std::sync::Arc<dyn Format> {
    fn with_serializer(
        &self,
        f: &mut dyn FnMut(&mut FormatSerializer) -> Result<(), FormatError>,
        context: &dyn Context,
    ) -> Result<Raw, FormatError> {
        (**self).with_serializer(f, context)
    }

    fn with_deserializer(
        &self,
        data: &[u8],
        f: &mut dyn FnMut(&mut FormatDeserializer) -> Result<(), FormatError>,
        ctx: &mut BoxContext,
    ) -> Result<(), FormatError> {
        (**self).with_deserializer(data, f, ctx)
    }

    fn clone_box(&self) -> Box<dyn Format> {
        (**self).clone_box()
    }

    fn format_type_id(&self) -> FormatTypeId {
        (**self).format_type_id()
    }
}

#[cfg(test)]
mod tests {
    use hitbox_core::CacheContext;

    use super::*;

    /// A large field followed by a small one, so the buffer grows past the
    /// serialized size while the small field is written.
    fn value() -> Vec<String> {
        vec!["x".repeat(100_000), "tail".to_owned()]
    }

    /// Length of the serialized value and capacity of the allocation behind it.
    fn len_and_capacity(format: &dyn Format) -> (usize, usize) {
        let raw = format
            .serialize(&value(), &CacheContext::default())
            .unwrap();
        let len = raw.len();
        let capacity = raw.try_into_mut().expect("buffer is not shared").capacity();
        (len, capacity)
    }

    #[test]
    fn serialized_buffers_have_no_spare_capacity() {
        let spare: Vec<_> = [&BincodeFormat as &dyn Format, &JsonFormat, &RonFormat]
            .into_iter()
            .map(|format| (format, len_and_capacity(format)))
            .filter(|(_, (len, capacity))| capacity != len)
            .collect();
        assert!(spare.is_empty(), "(format, (len, capacity)): {spare:?}");
    }
}
