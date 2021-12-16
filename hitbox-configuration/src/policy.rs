use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cache")]
pub(crate) enum CacheStatus {
    Enabled(InnerState),
    Disabled,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct InnerState {
    pub lock: LockStatus,
    pub stale: StaleStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum LockStatus {
    Local,
    Distributed,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum StaleStatus {
    Enabled,
    Disabled,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Policy {
    #[serde(flatten)]
    pub cache: CacheStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Conf {
    pub policies: HashMap<String, Policy>,
}
