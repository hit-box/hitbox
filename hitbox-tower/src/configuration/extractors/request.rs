use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestExtractor {
    Path { path: String },
    Method,
    Header { key: String },
    Query { key: String },
    //Body { statement: String },
}

pub struct ExtractorBuilder {
    predicates: Vec<RequestExtractor>,
}

impl ExtractorBuilder {
    pub fn new() -> Self {
        Self {
            predicates: Vec::new(),
        }
    }

    pub fn path(mut self, path: &str) -> Self {
        self.predicates.push(RequestExtractor::Path {
            path: path.to_owned(),
        });
        self
    }

    pub fn method(mut self) -> Self {
        self.predicates.push(RequestExtractor::Method);
        self
    }

    pub fn header(mut self, key: &str) -> Self {
        self.predicates.push(RequestExtractor::Header {
            key: key.to_owned(),
        });
        self
    }

    pub fn query(mut self, key: &str) -> Self {
        self.predicates.push(RequestExtractor::Query {
            key: key.to_owned(),
        });
        self
    }

    pub fn build(self) -> Vec<RequestExtractor> {
        self.predicates
    }
}

impl Default for ExtractorBuilder {
    fn default() -> Self {
        Self::new()
    }
}
