use hitbox_http::predicates::request::Method;
use http::Method as HttpMethod;
use serde::{Deserialize, Serialize};

use crate::{RequestPredicate, error::ConfigError};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum MethodOperation {
    Eq(String),
    In(Vec<String>),
}

impl MethodOperation {
    pub(crate) fn into_predicates<ReqBody: Send + 'static>(
        self,
        inner: RequestPredicate<ReqBody>,
    ) -> Result<RequestPredicate<ReqBody>, ConfigError> {
        match self {
            MethodOperation::Eq(method) => {
                let http_method = parse_method(&method)?;
                Ok(Box::new(Method::new(inner, http_method.as_str())?))
            }
            MethodOperation::In(methods) => {
                let http_methods = parse_methods(&methods)?;
                Ok(Box::new(Method::new_in(inner, http_methods)))
            }
        }
    }
}

fn parse_method(method: &str) -> Result<HttpMethod, ConfigError> {
    method
        .parse()
        .map_err(|e| ConfigError::InvalidMethod(method.to_string(), e))
}

fn parse_methods(methods: &[String]) -> Result<Vec<HttpMethod>, ConfigError> {
    methods.iter().map(|m| parse_method(m)).collect()
}
