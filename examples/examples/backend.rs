use serde::{Deserialize, Serialize};

trait Backend {
    fn get(&self, key: &str) -> Vec<u8>;
}

trait CacheBackend: Backend {
    fn get<T>(&self, key: &str) -> T {
        serde_json::from_slice(self.get(key).as_slice()).unwrap()
    }
}

impl<T> CacheBackend for T where T: Backend {}

#[derive(Serialize, Deserialize)]
struct CacheValue {
    name: String,
    ttl: u32,
}

struct Mem {}
impl Mem {
    fn new() -> Self {
        Mem {}
    }
}
impl Backend for Mem {
    fn get(&self, key: &str) -> Vec<u8> {
        serde_json::to_vec(&CacheValue {
            name: "mem".to_owned(),
            ttl: 42,
        })
        .unwrap()
    }
}

struct Redis {}
impl Redis {
    fn new() -> Self {
        Redis {}
    }
}
impl Backend for Redis {
    fn get(&self, key: &str) -> Vec<u8> {
        serde_json::to_vec(&CacheValue {
            name: "redis".to_owned(),
            ttl: 42,
        })
        .unwrap()
    }
}

fn main() {
    let backends = vec![Mem::new(), Redis::new()];
}
