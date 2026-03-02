//! gRPC proto field extractor (convenience wrapper).
//!
//! Provides [`grpc_proto_field`] which configures a
//! [`ProtoFieldExtractor`](hitbox_proto::extractors::field::ProtoFieldExtractor)
//! with the gRPC bytes extractor (strips the 5-byte frame header).

use hitbox_http::extractors::NeutralExtractor;
use hitbox_proto::extractors::field::ProtoFieldExtractor;
use prost_reflect::MessageDescriptor;

use crate::frame::grpc_bytes_extractor;

/// Creates a [`ProtoFieldExtractor`] pre-configured for gRPC framing.
///
/// This is a convenience constructor that uses [`grpc_bytes_extractor`] to strip
/// the gRPC 5-byte frame header before protobuf decoding.
///
/// # Examples
///
/// ```ignore
/// use hitbox_grpc::extractors::proto_field::grpc_proto_field;
///
/// let extractor = grpc_proto_field(msg_desc, "user_id");
/// ```
pub fn grpc_proto_field<S>(
    message_descriptor: MessageDescriptor,
    field_path: impl Into<String>,
) -> ProtoFieldExtractor<NeutralExtractor<S>> {
    ProtoFieldExtractor::new(message_descriptor, field_path, grpc_bytes_extractor)
}
