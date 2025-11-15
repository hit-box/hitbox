use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EnabledCacheConfig {
    pub ttl: Option<u32>,
    pub stale: Option<u32>,
    #[serde(default)]
    pub locks: LockConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub enum LockConfig {
    Enabled { concurrency: usize },
    #[default]
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum PolicyConfig {
    Enabled(EnabledCacheConfig),
    Disabled,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self::Enabled(EnabledCacheConfig {
            ttl: Some(5),
            stale: None,
            locks: LockConfig::Disabled,
        })
    }
}

