use serde::{Deserialize, Serialize};

use crate::cache::OverriddenCache;
use crate::request::Request;
use crate::response::Response;

#[derive(Debug, Serialize, Deserialize)]
pub struct Endpoint {
    #[serde(flatten)]
    cache: OverriddenCache,
    path: String,
    request: Option<Request>,
    response: Option<Response>,
}
