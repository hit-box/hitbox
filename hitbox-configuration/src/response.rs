use serde::{Deserialize, Serialize};

use crate::body::Body;
use crate::field::Field;
use crate::headers::Headers;
use crate::status_code::StatusCode;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    status_code: Option<StatusCode>,
    headers: Option<HashMap<String, Field>>,
    body: Option<Body>,
}
