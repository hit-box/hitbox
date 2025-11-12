use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::Schema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Default)]
pub enum MaybeUndefined<T> {
    Null,
    #[default]
    Undefined,
    Value(T),
}

// Implement JsonSchema manually for MaybeUndefined to treat it as Option<T>
impl<T: JsonSchema> JsonSchema for MaybeUndefined<T> {
    fn schema_name() -> String {
        format!("Nullable_{}", T::schema_name())
    }

    fn json_schema(schema_gen: &mut SchemaGenerator) -> Schema {
        // MaybeUndefined behaves like Option<T> in JSON Schema
        schema_gen.subschema_for::<Option<T>>()
    }
}

impl<T> MaybeUndefined<T> {
    /// Maps a `MaybeUndefined<T>` to `MaybeUndefined<U>` by applying a function
    /// to the contained nullable value
    #[inline]
    pub fn map<U, F: FnOnce(Option<T>) -> Option<U>>(self, f: F) -> MaybeUndefined<U> {
        match self {
            MaybeUndefined::Value(v) => match f(Some(v)) {
                Some(v) => MaybeUndefined::Value(v),
                None => MaybeUndefined::Null,
            },
            MaybeUndefined::Null => match f(None) {
                Some(v) => MaybeUndefined::Value(v),
                None => MaybeUndefined::Null,
            },
            MaybeUndefined::Undefined => MaybeUndefined::Undefined,
        }
    }
}

impl<T: Default> MaybeUndefined<T> {
    #[inline]
    pub fn unwrap_or_default(self) -> T {
        match self {
            MaybeUndefined::Value(v) => v,
            _ => Default::default(),
        }
    }
}

impl<T: Serialize> Serialize for MaybeUndefined<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            MaybeUndefined::Value(value) => value.serialize(serializer),
            _ => serializer.serialize_none(),
        }
    }
}

impl<'de, T> Deserialize<'de> for MaybeUndefined<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(|value| match value {
            Some(value) => MaybeUndefined::Value(value),
            None => MaybeUndefined::Null,
        })
    }
}
