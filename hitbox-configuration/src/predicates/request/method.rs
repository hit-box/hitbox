use hitbox_http::predicates::request::Method;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;

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
    ) -> RequestPredicate<ReqBody> {
        match self {
            MethodOperation::Eq(method) => Box::new(Method::new(inner, method.as_str()).unwrap()),
            MethodOperation::In(methods) => {
                let methods: Vec<http::Method> =
                    methods.into_iter().map(|m| m.parse().unwrap()).collect();
                Box::new(Method::new_in(inner, methods))
            }
        }
    }
}
