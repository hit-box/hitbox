use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cache")]
pub enum CacheStatus {
    Enabled(InnerState),
    Disabled,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InnerState {
    pub lock: LockStatus,
    pub stale: StaleStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LockStatus {
    Local,
    Distributed,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StaleStatus {
    Enabled,
    Disabled,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Policy {
    #[serde(flatten)]
    pub cache: CacheStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Conf {
    pub policies: HashMap<String, Policy>,
}
