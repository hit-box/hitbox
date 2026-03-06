use crate::Operation;

/// A single field specification: name + operation to check.
#[derive(Debug, Clone)]
pub struct FieldSpec {
    /// Field name (or dotted path for nested: "address.city").
    pub name: String,
    /// Operation to apply against the field value.
    pub operation: Operation,
}

/// Builder for specifying multiple fields to check in one decode pass.
///
/// # Example
///
/// ```ignore
/// use hitbox_protobuf::{FieldsBuilder, Operation, ProtoValue};
///
/// let fields = FieldsBuilder::new()
///     .field("user_id", Operation::Exists)
///     .field("role", Operation::Eq(ProtoValue::Enum("ADMIN".into())))
///     .field("age", Operation::Gt(ProtoValue::Int(18)))
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct FieldsBuilder {
    fields: Vec<FieldSpec>,
}

impl FieldsBuilder {
    /// Create an empty fields builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a field check.
    pub fn field(mut self, name: impl Into<String>, operation: Operation) -> Self {
        self.fields.push(FieldSpec {
            name: name.into(),
            operation,
        });
        self
    }

    /// Consume the builder and return the field specifications.
    pub fn build(self) -> Vec<FieldSpec> {
        self.fields
    }
}
