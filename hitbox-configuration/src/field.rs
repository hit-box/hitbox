use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "variants")]
pub(crate) enum FieldType {
    String,
    Enum(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Field {
    #[serde(flatten)]
    field_type: FieldType,
    exclude: Option<bool>,
}
