//! gRPC proto field predicate (convenience wrapper).
//!
//! Provides [`GrpcProtoField`] which configures a [`ProtoField`](hitbox_proto::predicates::field::ProtoField)
//! with the gRPC bytes extractor (strips the 5-byte frame header).

use hitbox::Neutral;
use hitbox_proto::predicates::field::{FieldOp, ProtoField};
use prost_reflect::MessageDescriptor;

use crate::frame::grpc_bytes_extractor;

/// Creates a [`ProtoField`] predicate pre-configured for gRPC framing.
///
/// This is a convenience constructor that uses [`grpc_bytes_extractor`] to strip
/// the gRPC 5-byte frame header before protobuf decoding.
///
/// # Examples
///
/// ```ignore
/// use hitbox_grpc::predicates::proto_field::grpc_proto_field;
/// use hitbox_proto::predicates::field::FieldOp;
/// use prost_reflect::Value;
///
/// let predicate = grpc_proto_field(msg_desc, "user_id", FieldOp::eq(Value::I64(42)));
/// ```
pub fn grpc_proto_field<S>(
    message_descriptor: MessageDescriptor,
    field_path: impl Into<String>,
    operation: FieldOp,
) -> ProtoField<Neutral<S>> {
    ProtoField::new(
        message_descriptor,
        field_path,
        operation,
        grpc_bytes_extractor,
    )
}
