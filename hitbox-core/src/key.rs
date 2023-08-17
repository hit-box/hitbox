#[derive(Debug)]
pub struct CacheKey {
    parts: Vec<KeyPart>,
    version: u32,
    prefix: String,
}

impl CacheKey {
    pub fn serialize(&self) -> String {
        let key = self
            .parts
            .iter()
            .map(|part| {
                format!(
                    "{}:{}",
                    part.key,
                    part.value.clone().unwrap_or("None".to_owned())
                )
            })
            .collect::<Vec<_>>()
            .join("::");
        format!("{}::{}::{}", self.prefix, self.version, key)
    }
}

#[derive(Debug)]
pub struct KeyPart {
    key: String,
    value: Option<String>,
}

impl KeyPart {
    pub fn new(key: String, value: Option<String>) -> Self {
        KeyPart { key, value }
    }
}

#[derive(Debug)]
pub struct KeyParts<T: Sized> {
    subject: T,
    parts: Vec<KeyPart>,
}

impl<T> KeyParts<T> {
    pub fn new(subject: T) -> Self {
        KeyParts {
            subject,
            parts: Vec::new(),
        }
    }

    pub fn push(&mut self, part: KeyPart) {
        self.parts.push(part)
    }

    pub fn append(&mut self, parts: &mut Vec<KeyPart>) {
        self.parts.append(parts)
    }

    pub fn into_cache_key(self) -> (T, CacheKey) {
        (
            self.subject,
            CacheKey {
                version: 0,
                prefix: String::new(),
                parts: self.parts,
            },
        )
    }
}
