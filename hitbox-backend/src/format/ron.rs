use bytes::Bytes;
use hitbox_core::{BoxContext, Raw};

use super::{Format, FormatDeserializer, FormatError, FormatSerializer, FormatTypeId};
use crate::context::Context;

/// RON (Rusty Object Notation) serialization format.
///
/// Human-readable format with Rust-like syntax. Produces more compact output
/// than JSON with better readability. Binary data uses `b'...'` notation.
#[derive(Debug, Clone, Copy, Default)]
pub struct RonFormat;

impl Format for RonFormat {
    fn with_serializer(
        &self,
        f: &mut dyn FnMut(&mut FormatSerializer) -> Result<(), FormatError>,
        _context: &dyn Context,
    ) -> Result<Raw, FormatError> {
        // RON serializer writes to std::fmt::Write (String), unlike JSON which uses std::io::Write (Vec<u8>)
        let mut buf = String::new();
        {
            let mut ser = ron::ser::Serializer::new(&mut buf, None)
                .map_err(|error| FormatError::Serialize(Box::new(error)))?;
            let mut erased = <dyn erased_serde::Serializer>::erase(&mut ser);
            let mut format_ser = FormatSerializer::Serde(&mut erased);
            f(&mut format_ser)?;
        }
        // Drop the growth slack: in-process backends store the buffer as is
        buf.shrink_to_fit();
        Ok(Bytes::from(buf.into_bytes()))
    }

    fn with_deserializer(
        &self,
        data: &[u8],
        f: &mut dyn FnMut(&mut FormatDeserializer) -> Result<(), FormatError>,
        _ctx: &mut BoxContext,
    ) -> Result<(), FormatError> {
        let s = std::str::from_utf8(data).map_err(|e| FormatError::Deserialize(Box::new(e)))?;
        let mut deserializer = ron::de::Deserializer::from_str(s)
            .map_err(|e| FormatError::Deserialize(Box::new(e)))?;
        let mut erased = <dyn erased_serde::Deserializer>::erase(&mut deserializer);
        let mut format_deserializer = FormatDeserializer::Serde(&mut erased);
        f(&mut format_deserializer)?;
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn Format> {
        Box::new(*self)
    }

    fn format_type_id(&self) -> FormatTypeId {
        FormatTypeId::Ron
    }
}
