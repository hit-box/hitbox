use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::backend::Backend;

#[derive(Debug, Serialize, Deserialize)]
pub struct Cache {
    #[serde(with = "humantime_serde")]
    pub ttl: Option<Duration>,
    pub prefix: Option<String>,
    pub version: Option<String>,
    pub backend: Option<String>,
    pub policy: Option<String>,
}
