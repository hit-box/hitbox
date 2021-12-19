use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Cache {
    backend: String,
    policy: String,
    upstream: String,
    #[serde(default = "ttl")]
    ttl: String,
    #[serde(default = "stale_ttl")]
    stale_ttl: String,
    prefix: Option<String>,
    version: Option<String>,
}

fn ttl() -> String {
    "5min".into()
}

fn stale_ttl() -> String {
    "4min".into()
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

impl OverriddenCache {
    pub(crate) fn merge(&self, cache: &Cache) -> Cache {
        Cache {
            backend: self.backend.clone().unwrap_or(cache.backend.clone()),
            policy: self.policy.clone().unwrap_or(cache.policy.clone()),
            upstream: self.upstream.clone().unwrap_or(cache.upstream.clone()),
            ttl: self.ttl.clone().unwrap_or(cache.ttl.clone()),
            stale_ttl: self.stale_ttl.clone().unwrap_or(cache.stale_ttl.clone()),
            prefix: self.backend.clone().or_else(|| cache.prefix.clone()),
            version: self.backend.clone().or_else(|| cache.version.clone()),
        }
    }
}