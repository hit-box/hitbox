/// A concrete protobuf value for comparison in predicate operations.
///
/// Maps to the scalar types and enum variant names from the protobuf
/// type system. Nested messages are accessed via dotted field paths.
#[derive(Debug, Clone, PartialEq)]
pub enum ProtoValue {
    /// UTF-8 string value.
    String(String),
    /// Signed integer (int32, int64, sint32, sint64, sfixed32, sfixed64).
    Int(i64),
    /// Unsigned integer (uint32, uint64, fixed32, fixed64).
    Uint(u64),
    /// Floating-point (float, double).
    Float(f64),
    /// Boolean value.
    Bool(bool),
    /// Raw bytes.
    Bytes(bytes::Bytes),
    /// Enum variant name (e.g., "ADMIN").
    Enum(String),
}

impl From<&str> for ProtoValue {
    fn from(s: &str) -> Self {
        ProtoValue::String(s.to_string())
    }
}

impl From<String> for ProtoValue {
    fn from(s: String) -> Self {
        ProtoValue::String(s)
    }
}

impl From<i64> for ProtoValue {
    fn from(v: i64) -> Self {
        ProtoValue::Int(v)
    }
}

impl From<i32> for ProtoValue {
    fn from(v: i32) -> Self {
        ProtoValue::Int(v as i64)
    }
}

impl From<u64> for ProtoValue {
    fn from(v: u64) -> Self {
        ProtoValue::Uint(v)
    }
}

impl From<u32> for ProtoValue {
    fn from(v: u32) -> Self {
        ProtoValue::Uint(v as u64)
    }
}

impl From<f32> for ProtoValue {
    fn from(v: f32) -> Self {
        ProtoValue::Float(v as f64)
    }
}

impl From<f64> for ProtoValue {
    fn from(v: f64) -> Self {
        ProtoValue::Float(v)
    }
}

impl From<Vec<u8>> for ProtoValue {
    fn from(v: Vec<u8>) -> Self {
        ProtoValue::Bytes(bytes::Bytes::from(v))
    }
}

impl From<&[u8]> for ProtoValue {
    fn from(v: &[u8]) -> Self {
        ProtoValue::Bytes(bytes::Bytes::copy_from_slice(v))
    }
}

impl From<bytes::Bytes> for ProtoValue {
    fn from(v: bytes::Bytes) -> Self {
        ProtoValue::Bytes(v)
    }
}

impl From<bool> for ProtoValue {
    fn from(v: bool) -> Self {
        ProtoValue::Bool(v)
    }
}
