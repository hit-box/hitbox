use hitbox::predicates::{Operation, Predicate};
use http::Request;
use hyper::Body;

pub struct Header {
    pub name: String,
    pub value: String,
    pub operation: Operation,
}

impl Predicate<Request<Body>> for Header {
    fn check(&self, request: &Request<Body>) -> bool {
        match self.operation {
            Operation::Eq => request
                .headers()
                .get(self.name.as_str())
                .map(|header_value| self.value.as_str() == header_value)
                .unwrap_or(false),
            _ => unimplemented!(),
        }
    }
}
