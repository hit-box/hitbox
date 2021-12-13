use serde::{Deserialize, Serialize};

use crate::body::Body;
use crate::field::Field;
use crate::headers::Headers;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    query: Option<HashMap<String, Field>>,
    headers: Option<HashMap<String, Field>>,
    body: Option<Body>,
}
