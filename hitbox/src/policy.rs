use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct EnabledCacheConfig {
    pub ttl: Option<u32>,
    pub stale: Option<u32>,
    pub locks: Option<LockConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct LockConfig {
    pub enabled: bool,
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
            locks: None,
        })
    }
}

impl EnabledCacheConfig {
    /// Returns the lock configuration if present.
    pub fn locks(&self) -> Option<&LockConfig> {
        self.locks.as_ref()
    }
}
