use serde::{Deserialize, Serialize};

use crate::field::Field;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Response {
    status_codes: Option<Vec<u16>>,
    headers: Option<HashMap<String, Field>>,
    #[serde(rename = "if")]
    body: Option<String>,
}
