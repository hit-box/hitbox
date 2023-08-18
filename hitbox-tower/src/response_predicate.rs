#[derive(Debug)]
pub enum ResponsePredicate {
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
