use std::borrow::Cow;

use crate::ProtoError;

/// Strips protocol-specific framing before protobuf decode.
///
/// gRPC wraps each protobuf message in a 5-byte length-prefix frame.
/// Twirp sends raw protobuf with no framing. Implementations extract
/// the raw protobuf bytes from a possibly-framed body.
///
/// # Implementing
///
/// Return `Cow::Borrowed` when no framing exists (zero-copy).
/// Return `Cow::Owned` when bytes must be sliced or transformed.
pub trait FrameDecoder: Send + Sync + 'static {
    /// Strip framing and return the raw protobuf bytes.
    fn decode<'a>(&self, body: &'a [u8]) -> Result<Cow<'a, [u8]>, ProtoError>;
}

/// Identity frame decoder — no framing applied.
///
/// Use for Twirp or any protocol that sends un-framed protobuf messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoFraming;

impl FrameDecoder for NoFraming {
    fn decode<'a>(&self, body: &'a [u8]) -> Result<Cow<'a, [u8]>, ProtoError> {
        Ok(Cow::Borrowed(body))
    }
}
