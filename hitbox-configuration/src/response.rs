use serde::{Deserialize, Serialize};

use crate::body::Body;
use crate::headers::Headers;
use crate::status_code::StatusCode;

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    status_code: Option<StatusCode>,
    headers: Option<Headers>,
    body: Option<Body>,
}

