use serde::{Deserialize, Serialize};
use crate::cache::Cache;

#[derive(Debug, Serialize, Deserialize)]
pub struct Endpoint {
    #[serde(flatten)]
    cache: Cache,
    path: String,
    request: Option<Request>,
    response: Option<Response>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {}
