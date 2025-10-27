use hitbox_http::FromBytes;
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::predicates::conditions::{Not, Or};
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;
use super::Expression;

// Use standard externally-tagged enum (serde default)
// YAML syntax: And: [...], Or: [...], Not: {...}
#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum Operation {
    And(Vec<Expression>),
    Or(Vec<Expression>),
    Not(Box<Expression>),
}

impl Operation {
    pub fn into_predicates<ReqBody>(
        &self,
        inner: RequestPredicate<ReqBody>,
    ) -> RequestPredicate<ReqBody>
    where
        ReqBody: HttpBody + FromBytes + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
        match self {
            Operation::Or(predicates) => {
                let mut iter = predicates.iter();
                match iter.next() {
                    None => inner,
                    Some(first) => {
                        let first_predicate =
                            first.into_predicates(Box::new(NeutralRequestPredicate::new()));
                        iter.fold(first_predicate, |acc, predicate| {
                            let predicate =
                                predicate
                                    .into_predicates(Box::new(
                                        NeutralRequestPredicate::<ReqBody>::new(),
                                    ));
                            Box::new(Or::new(
                                predicate,
                                acc,
                                Box::new(NeutralRequestPredicate::new()),
                            ))
                        })
                    }
                }
            }
            Operation::And(predicates) => predicates
                .iter()
                .rfold(inner, |inner, predicate| predicate.into_predicates(inner)),
            Operation::Not(expression) => {
                let predicate =
                    expression.into_predicates(Box::new(NeutralRequestPredicate::new()));
                Box::new(Not::new(predicate))
            }
        }
    }
}
