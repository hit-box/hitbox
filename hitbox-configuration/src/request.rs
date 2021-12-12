use serde::{Deserialize, Serialize};

use crate::body::Body;
use crate::headers::Headers;
use crate::query::Query;

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    query: Option<Query>,
    headers: Option<Headers>,
    body: Option<Body>,
}