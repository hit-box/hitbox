use crate::{FieldSpec, ProtoValue};

/// Matching operations for protobuf field values.
///
/// Operations are evaluated against a field extracted from a
/// `DynamicMessage`. For nested fields, use dotted paths in the
/// field name (e.g., `"address.city"`). For map fields, use the
/// key as a path segment (e.g., `"metadata.env"`).
///
/// # Example
///
/// ```ignore
/// use hitbox_protobuf::{Operation, ProtoValue};
///
/// // Simple field check
/// let op = Operation::Eq(ProtoValue::String("alice".into()));
///
/// // Nested field via dotted path in FieldsBuilder:
/// // .field("address.city", Operation::Eq("NYC".into()))
///
/// // Map field access:
/// // .field("metadata.env", Operation::Eq("production".into()))
/// ```
#[derive(Debug, Clone)]
pub enum Operation {
    /// Field exists and has a non-default value.
    Exists,
    /// Field value equals the specified value.
    Eq(ProtoValue),
    /// Field value does not equal the specified value.
    NotEq(ProtoValue),
    /// Field value is greater than (numeric types only).
    Gt(ProtoValue),
    /// Field value is less than (numeric types only).
    Lt(ProtoValue),
    /// String or bytes field contains the substring.
    Contains(String),
    /// String field matches the regex pattern.
    Regex(regex::Regex),
    /// Field value is one of the specified values.
    In(Vec<ProtoValue>),
    /// At least one element in a repeated field matches.
    Any(Box<ElementMatcher>),
    /// All elements in a repeated field match.
    All(Box<ElementMatcher>),
}

impl Operation {
    /// Create an `Any` operation from a matcher.
    pub fn any(matcher: impl Into<ElementMatcher>) -> Self {
        Operation::Any(Box::new(matcher.into()))
    }

    /// Create an `All` operation from a matcher.
    pub fn all(matcher: impl Into<ElementMatcher>) -> Self {
        Operation::All(Box::new(matcher.into()))
    }
}

/// Matcher for elements inside a repeated field.
///
/// Use [`ValueMatcher`] for scalar repeated fields (`repeated string`, `repeated int32`)
/// and [`FieldsBuilder`](crate::FieldsBuilder) for message repeated fields (`repeated Item`).
///
/// # Examples
///
/// ```ignore
/// use hitbox_protobuf::{Operation, ValueMatcher, FieldsBuilder, ProtoValue};
///
/// // Scalar: any tag equals "important"
/// Operation::any(ValueMatcher::new(Operation::Eq("important".into())));
///
/// // Message: any item where name = "widget" AND quantity > 10
/// Operation::any(
///     FieldsBuilder::new()
///         .field("name", Operation::Eq("widget".into()))
///         .field("quantity", Operation::Gt(ProtoValue::Int(10)))
///         .build(),
/// );
/// ```
#[derive(Debug, Clone)]
pub enum ElementMatcher {
    /// Match scalar elements by value.
    Value(ValueMatcher),
    /// Match message elements by their fields (AND semantics).
    Fields(Vec<FieldCheck>),
}

impl From<ValueMatcher> for ElementMatcher {
    fn from(v: ValueMatcher) -> Self {
        ElementMatcher::Value(v)
    }
}

impl From<Vec<FieldSpec>> for ElementMatcher {
    fn from(specs: Vec<FieldSpec>) -> Self {
        ElementMatcher::Fields(
            specs
                .into_iter()
                .map(|s| FieldCheck {
                    name: s.name,
                    operation: Box::new(s.operation),
                })
                .collect(),
        )
    }
}

/// A field check inside an [`ElementMatcher`], with boxed operation to break recursion.
#[derive(Debug, Clone)]
pub struct FieldCheck {
    /// Field name (or dotted path for nested: "address.city").
    pub name: String,
    /// Operation to apply against the field value.
    pub operation: Box<Operation>,
}

/// Matcher for a scalar element inside a repeated field.
///
/// Wraps an [`Operation`] to apply against each element value.
///
/// # Example
///
/// ```ignore
/// use hitbox_protobuf::{Operation, ValueMatcher};
///
/// // Any tag equals "important"
/// let op = Operation::any(ValueMatcher::new(Operation::Eq("important".into())));
/// ```
#[derive(Debug, Clone)]
pub struct ValueMatcher {
    pub(crate) operation: Box<Operation>,
}

impl ValueMatcher {
    /// Create a new value matcher with the given operation.
    pub fn new(operation: Operation) -> Self {
        Self {
            operation: Box::new(operation),
        }
    }
}
