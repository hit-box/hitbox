use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "variants")]
pub enum FieldType {
    String,
    Enum(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Field {
    #[serde(flatten)]
    field_type: FieldType,
}
