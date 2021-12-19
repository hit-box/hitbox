use serde::{Deserialize, Serialize};

use crate::field::Field;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Request {
    query: Option<HashMap<String, Field>>,
    headers: Option<HashMap<String, Field>>,
    body: Option<String>,
}
