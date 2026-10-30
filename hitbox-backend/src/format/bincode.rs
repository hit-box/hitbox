use bytes::Bytes;
use hitbox_core::{BoxContext, Raw};

use super::{
    BincodeDecoder, BincodeEncoder, Format, FormatDeserializer, FormatError, FormatSerializer,
    FormatTypeId,
};
use crate::context::Context;

/// Writer wrapper for bincode serialization.
///
/// Bincode's internal `VecWriter` is not publicly exported, so this provides
/// the same functionality. This type is exposed in `FormatSerializer::Bincode`
/// but cannot be constructed outside this crate.
#[derive(Default)]
pub(crate) struct BincodeVecWriter(Vec<u8>);

impl BincodeVecWriter {
    pub(crate) fn new() -> Self {
        Self(Vec::new())
    }

    pub(crate) fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

impl ::bincode::enc::write::Writer for BincodeVecWriter {
    fn write(&mut self, bytes: &[u8]) -> Result<(), ::bincode::error::EncodeError> {
        self.0.extend_from_slice(bytes);
        Ok(())
    }
}

/// Fast, compact binary serialization.
///
/// Produces the smallest output among serde-based formats with good performance.
/// Not human-readable. Uses bincode's standard configuration.
#[derive(Debug, Clone, Copy, Default)]
pub struct BincodeFormat;

impl Format for BincodeFormat {
    fn with_serializer(
        &self,
        f: &mut dyn FnMut(&mut FormatSerializer) -> Result<(), FormatError>,
        _context: &dyn Context,
    ) -> Result<Raw, FormatError> {
        let writer = BincodeVecWriter::new();
        let config = ::bincode::config::standard();
        let mut encoder = ::bincode::enc::EncoderImpl::new(writer, config);

        // Create FormatSerializer::Bincode variant with opaque wrapper
        let mut format_ser = FormatSerializer::Bincode(BincodeEncoder(&mut encoder));
        f(&mut format_ser)?;

        // Extract the buffer from the encoder and drop its growth slack:
        // in-process backends store it as is
        let mut buf = encoder.into_writer().into_vec();
        buf.shrink_to_fit();
        Ok(Bytes::from(buf))
    }

    fn with_deserializer(
        &self,
        data: &[u8],
        f: &mut dyn FnMut(&mut FormatDeserializer) -> Result<(), FormatError>,
        _ctx: &mut BoxContext,
    ) -> Result<(), FormatError> {
        use ::bincode::de::read::SliceReader;

        let reader = SliceReader::new(data);
        let config = ::bincode::config::standard();
        let context = (); // Context for the decoder
        let mut decoder = ::bincode::de::DecoderImpl::new(reader, config, context);

        // Create FormatDeserializer::Bincode variant with opaque wrapper
        let mut format_deserializer = FormatDeserializer::Bincode(BincodeDecoder(&mut decoder));
        f(&mut format_deserializer)?;

        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Format> {
        Box::new(*self)
    }

    fn format_type_id(&self) -> FormatTypeId {
        FormatTypeId::Bincode
    }
}
