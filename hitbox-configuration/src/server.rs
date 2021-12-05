use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Protocol {
    Http,
    Https,
    Grpc,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u16,
    pub proto: Protocol,
}
