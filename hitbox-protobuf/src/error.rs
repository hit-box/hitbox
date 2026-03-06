/// Errors from protobuf operations in Hitbox.
#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    /// Failed to decode protobuf message bytes.
    #[error("protobuf decode error: {0}")]
    DecodeError(#[from] prost::DecodeError),
    /// Failed to decode a descriptor pool from bytes.
    #[error("descriptor pool error: {0}")]
    DescriptorPoolError(#[from] prost_reflect::DescriptorError),
    /// Message or service descriptor not found in the pool.
    #[error("descriptor not found: {0}")]
    DescriptorNotFound(String),
    // TODO: replace with a concrete error type when hitbox-grpc FrameDecoder is implemented.
    /// Protocol-specific frame decoding failed (e.g., gRPC 5-byte prefix).
    #[error("frame decode error: {0}")]
    FrameError(String),
    // TODO: replace with a concrete error type when proto_files feature is stabilized.
    /// Failed to parse .proto files.
    #[error("proto file error: {0}")]
    ProtoFileError(String),
}
