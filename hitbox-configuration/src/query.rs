use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "variants")]
pub enum QueryType {
    Integer,
    String,
    Enum(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Query {
    #[serde(flatten)]
    query_type: QueryType,
}
