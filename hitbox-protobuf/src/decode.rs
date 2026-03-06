use prost_reflect::{
    DynamicMessage, FieldDescriptor, Kind, MapKey, MessageDescriptor, ReflectMessage, Value,
};

use crate::operation::{ElementMatcher, FieldCheck};
use crate::{Operation, ProtoError, ProtoValue};

/// Decode raw protobuf bytes into a DynamicMessage using reflection.
pub fn decode_message(
    descriptor: &MessageDescriptor,
    bytes: &[u8],
) -> Result<DynamicMessage, ProtoError> {
    Ok(DynamicMessage::decode(descriptor.clone(), bytes)?)
}

/// Extract a field value from a DynamicMessage by name.
///
/// Supports dotted paths for nested messages (e.g., "address.city")
/// and map key access (e.g., "metadata.env" where metadata is a map).
/// Returns an owned `Value` because `get_field` returns `Cow<Value>`.
pub fn extract_field(message: &DynamicMessage, field_path: &str) -> Option<Value> {
    let mut parts = field_path.splitn(2, '.');
    let field_name = parts.next()?;
    let rest = parts.next();

    let field_desc = message.descriptor().get_field_by_name(field_name)?;
    let value = message.get_field(&field_desc);

    match rest {
        Some(remaining) => match &*value {
            Value::Message(inner) => extract_field(inner, remaining),
            Value::Map(map) => extract_map_value(&field_desc, map, remaining),
            _ => None,
        },
        None => Some(value.into_owned()),
    }
}

/// Extract a value from a map by key, with optional further path traversal.
fn extract_map_value(
    field_desc: &FieldDescriptor,
    map: &std::collections::HashMap<MapKey, Value>,
    remaining_path: &str,
) -> Option<Value> {
    let mut parts = remaining_path.splitn(2, '.');
    let key_str = parts.next()?;
    let rest = parts.next();

    let map_key = parse_map_key(field_desc, key_str)?;
    let map_value = map.get(&map_key)?;

    match rest {
        Some(further) => {
            if let Value::Message(inner) = map_value {
                extract_field(inner, further)
            } else {
                None
            }
        }
        None => Some(map_value.clone()),
    }
}

/// Parse a string path segment into a typed MapKey based on the map's key type.
fn parse_map_key(field_desc: &FieldDescriptor, key_str: &str) -> Option<MapKey> {
    let Kind::Message(entry_desc) = field_desc.kind() else {
        return None;
    };
    let key_field = entry_desc.get_field_by_name("key")?;

    match key_field.kind() {
        Kind::String => Some(MapKey::String(key_str.to_string())),
        Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => key_str.parse::<i32>().ok().map(MapKey::I32),
        Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => key_str.parse::<i64>().ok().map(MapKey::I64),
        Kind::Uint32 | Kind::Fixed32 => key_str.parse::<u32>().ok().map(MapKey::U32),
        Kind::Uint64 | Kind::Fixed64 => key_str.parse::<u64>().ok().map(MapKey::U64),
        Kind::Bool => key_str.parse::<bool>().ok().map(MapKey::Bool),
        _ => None,
    }
}

/// Check if a field has a non-default value (for `Operation::Exists`).
///
/// Supports dotted paths for nested messages (e.g., "address.city").
pub fn field_has_value(message: &DynamicMessage, field_path: &str) -> bool {
    extract_field(message, field_path).is_some_and(|v| !is_default_value(&v))
}

fn is_default_value(value: &Value) -> bool {
    match value {
        Value::Bool(b) => !b,
        Value::I32(v) => *v == 0,
        Value::I64(v) => *v == 0,
        Value::U32(v) => *v == 0,
        Value::U64(v) => *v == 0,
        Value::F32(v) => *v == 0.0,
        Value::F64(v) => *v == 0.0,
        Value::String(s) => s.is_empty(),
        Value::Bytes(b) => b.is_empty(),
        Value::EnumNumber(n) => *n == 0,
        Value::List(l) => l.is_empty(),
        Value::Map(m) => m.is_empty(),
        _ => false,
    }
}

/// Check if an Operation matches a field in the message.
pub fn check_operation(operation: &Operation, message: &DynamicMessage, field_name: &str) -> bool {
    match operation {
        Operation::Exists => field_has_value(message, field_name),
        Operation::Any(matcher) => {
            let Some(Value::List(items)) = extract_field(message, field_name) else {
                return false;
            };
            items.iter().any(|v| element_matches(matcher, v))
        }
        Operation::All(matcher) => {
            let Some(Value::List(items)) = extract_field(message, field_name) else {
                return false;
            };
            !items.is_empty() && items.iter().all(|v| element_matches(matcher, v))
        }
        op => {
            let Some(value) = extract_field(message, field_name) else {
                return false;
            };
            let field_desc = resolve_field_descriptor(message, field_name);
            let Some(proto_value) = value_to_proto_value_ext(&value, field_desc.as_ref()) else {
                return false;
            };
            match_value(op, &proto_value)
        }
    }
}

/// Check if a single list element matches an ElementMatcher.
fn element_matches(matcher: &ElementMatcher, value: &Value) -> bool {
    match matcher {
        ElementMatcher::Value(vm) => {
            let Some(pv) = value_to_proto_value(value) else {
                return false;
            };
            match_value(&vm.operation, &pv)
        }
        ElementMatcher::Fields(checks) => match value {
            Value::Message(msg) => checks
                .iter()
                .all(|FieldCheck { name, operation }| check_operation(operation, msg, name)),
            _ => false,
        },
    }
}

/// Resolve the FieldDescriptor for the leaf field in a dotted path.
///
/// Used to look up enum variant names from enum numbers.
pub fn resolve_field_descriptor(
    message: &DynamicMessage,
    field_path: &str,
) -> Option<FieldDescriptor> {
    let mut parts = field_path.splitn(2, '.');
    let field_name = parts.next()?;
    let rest = parts.next();

    let field_desc = message.descriptor().get_field_by_name(field_name)?;

    match rest {
        Some(remaining) => {
            let value = message.get_field(&field_desc);
            match &*value {
                Value::Message(inner) => resolve_field_descriptor(inner, remaining),
                _ => None,
            }
        }
        None => Some(field_desc),
    }
}

/// Convert a prost_reflect Value to our ProtoValue for comparison.
pub fn value_to_proto_value(value: &Value) -> Option<ProtoValue> {
    value_to_proto_value_ext(value, None)
}

/// Convert a prost_reflect Value to ProtoValue, with optional enum name resolution.
fn value_to_proto_value_ext(
    value: &Value,
    field_desc: Option<&FieldDescriptor>,
) -> Option<ProtoValue> {
    match value {
        Value::Bool(b) => Some(ProtoValue::Bool(*b)),
        Value::I32(v) => Some(ProtoValue::Int(*v as i64)),
        Value::I64(v) => Some(ProtoValue::Int(*v)),
        Value::U32(v) => Some(ProtoValue::Uint(*v as u64)),
        Value::U64(v) => Some(ProtoValue::Uint(*v)),
        Value::F32(v) => Some(ProtoValue::Float(*v as f64)),
        Value::F64(v) => Some(ProtoValue::Float(*v)),
        Value::String(s) => Some(ProtoValue::String(s.clone())),
        Value::Bytes(b) => Some(ProtoValue::Bytes(bytes::Bytes::copy_from_slice(b))),
        Value::EnumNumber(n) => {
            if let Some(desc) = field_desc
                && let Kind::Enum(enum_desc) = desc.kind()
                && let Some(variant) = enum_desc.get_value(*n)
            {
                return Some(ProtoValue::Enum(variant.name().to_string()));
            }
            Some(ProtoValue::Int(*n as i64))
        }
        _ => None,
    }
}

/// Convert a prost_reflect Value to a string for cache key generation.
pub fn value_to_key_string(value: &Value) -> Option<String> {
    value_to_key_string_ext(value, None)
}

/// Convert a prost_reflect Value to a string for cache key generation,
/// with optional field descriptor for enum name resolution.
pub fn value_to_key_string_ext(
    value: &Value,
    field_desc: Option<&FieldDescriptor>,
) -> Option<String> {
    match value {
        Value::Bool(b) => Some(b.to_string()),
        Value::I32(v) => Some(v.to_string()),
        Value::I64(v) => Some(v.to_string()),
        Value::U32(v) => Some(v.to_string()),
        Value::U64(v) => Some(v.to_string()),
        Value::F32(v) => Some(v.to_string()),
        Value::F64(v) => Some(v.to_string()),
        Value::String(s) => Some(s.clone()),
        Value::Bytes(b) => {
            let hex: String = b.iter().map(|byte| format!("{byte:02x}")).collect();
            Some(hex)
        }
        Value::EnumNumber(n) => {
            if let Some(desc) = field_desc
                && let Kind::Enum(enum_desc) = desc.kind()
                && let Some(variant) = enum_desc.get_value(*n)
            {
                return Some(variant.name().to_string());
            }
            Some(n.to_string())
        }
        Value::List(items) => {
            let mut parts: Vec<String> = items.iter().filter_map(value_to_key_string).collect();
            if parts.is_empty() {
                return None;
            }
            parts.sort();
            Some(parts.join(","))
        }
        _ => None,
    }
}

fn match_value(operation: &Operation, value: &ProtoValue) -> bool {
    match operation {
        Operation::Exists => true, // handled separately
        Operation::Eq(expected) => value == expected,
        Operation::NotEq(expected) => value != expected,
        Operation::Gt(expected) => {
            compare_numeric(value, expected) == Some(std::cmp::Ordering::Greater)
        }
        Operation::Lt(expected) => {
            compare_numeric(value, expected) == Some(std::cmp::Ordering::Less)
        }
        Operation::Contains(substring) => match value {
            ProtoValue::String(s) => s.contains(substring.as_str()),
            ProtoValue::Bytes(b) => {
                let needle = substring.as_bytes();
                b.windows(needle.len()).any(|w| w == needle)
            }
            _ => false,
        },
        Operation::Regex(re) => match value {
            ProtoValue::String(s) => re.is_match(s),
            _ => false,
        },
        Operation::In(variants) => variants.contains(value),
        // Any/All are handled in check_operation, not here
        Operation::Any(_) | Operation::All(_) => false,
    }
}

fn compare_numeric(a: &ProtoValue, b: &ProtoValue) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (ProtoValue::Int(a), ProtoValue::Int(b)) => Some(a.cmp(b)),
        (ProtoValue::Uint(a), ProtoValue::Uint(b)) => Some(a.cmp(b)),
        (ProtoValue::Float(a), ProtoValue::Float(b)) => a.partial_cmp(b),
        (ProtoValue::Int(a), ProtoValue::Uint(b)) => {
            if *a < 0 {
                Some(std::cmp::Ordering::Less)
            } else {
                (*a as u64).partial_cmp(b)
            }
        }
        (ProtoValue::Uint(a), ProtoValue::Int(b)) => {
            if *b < 0 {
                Some(std::cmp::Ordering::Greater)
            } else {
                a.partial_cmp(&(*b as u64))
            }
        }
        _ => None,
    }
}
