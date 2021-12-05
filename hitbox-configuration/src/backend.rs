use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Backend {
    pub host: Option<String>,
    pub port: Option<usize>,
    pub database: Option<u16>,
    pub max_size: Option<String>,
}
