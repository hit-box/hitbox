use serde::{Deserialize, Serialize};

use crate::body::Body;
use crate::headers::Headers;
use crate::query::Query;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    query: Option<HashMap<String, Query>>,
    headers: Option<Headers>,
    body: Option<Body>,
}
