#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CacheKey {
    parts: Vec<KeyPart>,
    version: u32,
    prefix: String,
}

impl CacheKey {
    pub fn parts(&self) -> impl Iterator<Item = &KeyPart> {
        self.parts.iter()
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    pub fn from_str(key: &str, value: &str) -> Self {
        CacheKey {
            parts: vec![KeyPart::new(key, Some(value))],
            version: 0,
            prefix: "".to_owned(),
        }
    }

    pub fn from_slice(parts: &[(&str, &str)]) -> Self {
        CacheKey {
            parts: parts
                .iter()
                .map(|(key, value)| {
                    let val = if *value == "null" {
                        None
                    } else {
                        Some(*value)
                    };
                    KeyPart::new(key, val)
                })
                .collect(),
            version: 0,
            prefix: "".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub struct KeyPart {
    key: String,
    value: Option<String>,
}

impl KeyPart {
    pub fn new<K: ToString, V: ToString>(key: K, value: Option<V>) -> Self {
        KeyPart {
            key: key.to_string(),
            value: value.as_ref().map(V::to_string),
        }
    }

    pub fn key(&self) -> &String {
        &self.key
    }

    pub fn value(&self) -> &Option<String> {
        &self.value
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
