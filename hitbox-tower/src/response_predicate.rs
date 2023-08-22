use serde::{Deserialize, Serialize};

mod status_code {
    use http::StatusCode;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(status_code: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = status_code.to_string();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<StatusCode, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        StatusCode::from_bytes(s.as_bytes()).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ResponsePredicate {
    #[serde(with = "status_code")]
    StatusCode { code: http::StatusCode },
    //Body { statement: String },
}

pub struct ResponsePredicateBuilder {
    predicates: Vec<ResponsePredicate>,
}

impl ResponsePredicateBuilder {
    pub fn new() -> Self {
        Self {
            predicates: Vec::new(),
        }
    }

    pub fn status_code(mut self, code: http::StatusCode) -> Self {
        self.predicates.push(ResponsePredicate::StatusCode { code });
        self
    }

    pub fn build(self) -> Vec<ResponsePredicate> {
        self.predicates
    }
}

impl Default for ResponsePredicateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
