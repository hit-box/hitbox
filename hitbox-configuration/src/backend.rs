use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Backends {
    Redis(RedisBackend),
    InMemory(InMemoryBackend),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedisBackend {
    host: String,
    port: u16,
    database: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InMemoryBackend {
    max_size: String,
}
