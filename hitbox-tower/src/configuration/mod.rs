pub mod builder;
mod endpoint;
mod extractors;
mod predicates;
pub mod serializers;

pub use endpoint::EndpointConfig;
pub use extractors::{ExtractorBuilder, RequestExtractor};
pub use predicates::{
    RequestPredicate, RequestPredicateBuilder, ResponsePredicate, ResponsePredicateBuilder,
};

pub mod predicate {
    pub mod request {
        use crate::configuration::predicates::RequestPredicateBuilder;

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
        use crate::configuration::predicates::ResponsePredicateBuilder;

        pub fn status_code(code: http::StatusCode) -> ResponsePredicateBuilder {
            ResponsePredicateBuilder::new().status_code(code)
        }
    }
}

pub mod extractor {
    use crate::configuration::extractors::ExtractorBuilder;

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
