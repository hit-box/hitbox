use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::backend::Backend;

#[derive(Debug, Serialize, Deserialize)]
pub struct Cache {
    ttl: Option<String>,
    stale_ttl: Option<String>,
    prefix: Option<String>,
    version: Option<String>,
    backend: Option<String>,
    policy: Option<String>,
    upstream: Option<String>,
}
