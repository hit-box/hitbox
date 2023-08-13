#[derive(Clone)]
pub struct Config {
    pub query: (String, String),
}

impl Config {
    pub fn new() -> Self {
        Self { query: (String::from("key"), String::from("value")) }
    }
}

