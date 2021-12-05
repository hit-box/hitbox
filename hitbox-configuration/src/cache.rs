use serde::{Deserialize, Serialize};

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
