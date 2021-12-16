use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum Protocol {
    Http,
    Https,
    Grpc,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Server {
    pub host: String,
    pub port: u16,
    pub proto: Protocol,
}
