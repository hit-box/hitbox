use std::collections::HashMap;
use tokio::sync::RwLock;

struct MemBackend {
    storage: RwLock<HashMap<String, String>>,
    format: Format,
}

impl MemBackend {
    fn new(format: Format) -> Self {
        let mut storage = HashMap::new();
        storage.insert("key1".to_owned(), "42".to_owned());
        MemBackend {
            storage: RwLock::new(storage),
            format,
        }
    }
}

impl Backend for MemBackend {
    async fn get_raw(&self, key: &str) -> Option<String> {
        let lock = self.storage.read().await;
        lock.get(key).cloned()
    }

    fn serializer(&self, raw: String) -> Box<dyn Deserializer> {
        // Box::new(<dyn Serializer>::erase(&mut self.serializer));
        unimplemented!()
    }
}

enum Format {
    Json,
    Cbor,
}

impl Format {
    fn serialize(&self) {}
}

// struct FormatRegistry {
//     json: serde_json::Serializer<Vec<u8>>,
//     registry: HashMap<&'static str, Arc<>>
//     formats: HashMap<&'static str, Box<dyn Serializer>>,
// }
//
// impl FormatRegistry {
//     fn new() -> Self {
//         let mut formats = HashMap::new();
//         let mut json = serde_json::Serializer::new(Vec::with_capacity(128));
//         formats.insert("json", Box::new(<dyn Serializer>::erase(&mut json)) as _);
//         FormatRegistry { json, formats }
//     }
// }

#[tokio::main]
async fn main() {
    let storage = MemBackend::new(Format::Json);
    let value = storage.get::<u8>("key1").await;
    dbg!(value);

    let backend: Box<dyn ErasedBackend> = Box::new(storage);
    let value = backend.get::<u8>("key1").await;
    dbg!(value);
}
