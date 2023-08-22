use serde::{Deserialize, Serialize};

mod method {
    use http::Method;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(method: &Method, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = method.to_string();
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Method, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Method::from_bytes(s.as_bytes()).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RequestPredicate {
    Path {
        path: String,
    },
    #[serde(with = "method")]
    Method {
        method: http::Method,
    },
    Header {
        key: String,
        value: String,
    },
    Query {
        key: String,
        value: String,
    },
    //Body { statement: String },
}

pub struct RequestPredicateBuilder {
    predicates: Vec<RequestPredicate>,
}

impl RequestPredicateBuilder {
    pub fn new() -> Self {
        Self {
            predicates: Vec::new(),
        }
    }

    pub fn path(mut self, path: &str) -> Self {
        self.predicates.push(RequestPredicate::Path {
            path: path.to_owned(),
        });
        self
    }

    pub fn method(mut self, method: http::Method) -> Self {
        self.predicates.push(RequestPredicate::Method { method });
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.predicates.push(RequestPredicate::Header {
            key: key.to_owned(),
            value: value.to_owned(),
        });
        self
    }

    pub fn query(mut self, key: &str, value: &str) -> Self {
        self.predicates.push(RequestPredicate::Query {
            key: key.to_owned(),
            value: value.to_owned(),
        });
        self
    }

    pub fn build(self) -> Vec<RequestPredicate> {
        self.predicates
    }
}

impl Default for RequestPredicateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
