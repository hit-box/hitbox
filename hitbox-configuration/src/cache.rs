use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Cache {
    backend: String,
    policy: String,
    upstream: String,
    ttl: Option<String>,
    stale_ttl: Option<String>,
    prefix: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OverriddenCache {
    ttl: Option<String>,
    stale_ttl: Option<String>,
    prefix: Option<String>,
    version: Option<String>,
    backend: Option<String>,
    policy: Option<String>,
    upstream: Option<String>,
}
