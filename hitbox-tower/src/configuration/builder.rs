use crate::configuration::{
    ExtractorBuilder, RequestExtractor, RequestPredicate, RequestPredicateBuilder,
    ResponsePredicate, ResponsePredicateBuilder,
};
use crate::EndpointConfig;
use hitbox::policy::PolicyConfig;

#[derive(Debug)]
pub struct EndpointConfigBuilder {
    pub request_predicates: Vec<RequestPredicate>,
    pub response_predicates: Vec<ResponsePredicate>,
    pub extractors: Vec<RequestExtractor>,
    pub policy: PolicyConfig,
}

impl EndpointConfigBuilder {
    pub fn new() -> Self {
        Self {
            request_predicates: Vec::new(),
            response_predicates: Vec::new(),
            extractors: Vec::new(),
            policy: Default::default(),
        }
    }

    pub fn disable(self) -> Self {
        Self {
            request_predicates: self.request_predicates,
            response_predicates: self.response_predicates,
            extractors: self.extractors,
            policy: PolicyConfig::Disabled,
        }
    }

    pub fn request(self, predicates: RequestPredicateBuilder) -> Self {
        Self {
            request_predicates: predicates.build(),
            response_predicates: self.response_predicates,
            extractors: self.extractors,
            policy: self.policy,
        }
    }

    pub fn response(self, predicates: ResponsePredicateBuilder) -> Self {
        Self {
            request_predicates: self.request_predicates,
            response_predicates: predicates.build(),
            extractors: self.extractors,
            policy: self.policy,
        }
    }

    pub fn cache_key(self, extractors: ExtractorBuilder) -> Self {
        Self {
            request_predicates: self.request_predicates,
            response_predicates: self.response_predicates,
            extractors: extractors.build(),
            policy: self.policy,
        }
    }

    pub fn build(self) -> EndpointConfig {
        EndpointConfig {
            request_predicates: self.request_predicates,
            response_predicates: self.response_predicates,
            extractors: self.extractors,
            policy: self.policy,
        }
    }
}

impl Default for EndpointConfigBuilder {
    fn default() -> Self {
        Self {
            request_predicates: Vec::new(),
            response_predicates: vec![ResponsePredicate::StatusCode {
                code: http::StatusCode::OK,
            }],
            extractors: vec![
                RequestExtractor::Path {
                    path: String::from("{path}*"),
                },
                RequestExtractor::Method,
            ],
            policy: Default::default(),
        }
    }
}
