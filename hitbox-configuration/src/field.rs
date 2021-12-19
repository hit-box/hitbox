use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "variants")]
pub(crate) enum FieldType {
    String,
    Enum(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Field {
    #[serde(flatten)]
    field_type: FieldType,
    exclude: Option<bool>,
}
