pub mod config;
pub mod future;
pub mod layer;
pub mod request_extractor;
pub mod request_predicate;
pub mod response_predicate;
pub mod service;

pub use ::http::{Method, StatusCode};
pub use layer::Cache;

pub mod predicate {
    pub mod request {
        use crate::request_predicate::RequestPredicateBuilder;

        pub fn path(path: &str) -> RequestPredicateBuilder {
            RequestPredicateBuilder::new().path(path)
        }

        pub fn method(method: http::Method) -> RequestPredicateBuilder {
            RequestPredicateBuilder::new().method(method)
        }

        pub fn header(key: &str, value: &str) -> RequestPredicateBuilder {
            RequestPredicateBuilder::new().header(key, value)
        }

        pub fn query(key: &str, value: &str) -> RequestPredicateBuilder {
            RequestPredicateBuilder::new().query(key, value)
        }
    }

    pub mod response {
        use crate::response_predicate::ResponsePredicateBuilder;

        pub fn status_code(code: http::StatusCode) -> ResponsePredicateBuilder {
            ResponsePredicateBuilder::new().status_code(code)
        }
    }
}

pub mod extractor {
    use crate::request_extractor::ExtractorBuilder;

    pub fn path(path: &str) -> ExtractorBuilder {
        ExtractorBuilder::new().path(path)
    }

    pub fn method() -> ExtractorBuilder {
        ExtractorBuilder::new().method()
    }

    pub fn header(key: &str) -> ExtractorBuilder {
        ExtractorBuilder::new().header(key)
    }

    pub fn query(key: &str) -> ExtractorBuilder {
        ExtractorBuilder::new().query(key)
    }
}
