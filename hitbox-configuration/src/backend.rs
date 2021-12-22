use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum Backend {
    Redis(RedisBackend),
    InMemory(InMemoryBackend),
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RedisBackend {
    host: String,
    port: u16,
    database: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct InMemoryBackend {
    max_size: String,
}
