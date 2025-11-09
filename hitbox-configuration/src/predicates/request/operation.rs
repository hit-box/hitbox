use hitbox_http::FromBytes;
use hitbox_http::predicates::NeutralRequestPredicate;
use hitbox_http::predicates::conditions::{Not, Or};
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use super::Expression;
use crate::{RequestPredicate, error::ConfigError};

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
        self,
        inner: RequestPredicate<ReqBody>,
    ) -> Result<RequestPredicate<ReqBody>, ConfigError>
    where
        ReqBody: HttpBody + FromBytes + hitbox_http::FromChunks<ReqBody::Error> + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
        match self {
            Operation::Or(predicates) => {
                let mut iter = predicates.into_iter();
                match iter.next() {
                    None => Ok(inner),
                    Some(first) => {
                        let first_predicate =
                            first.into_predicates(Box::new(NeutralRequestPredicate::new()))?;
                        iter.try_fold(first_predicate, |acc, predicate| {
                            let predicate =
                                predicate
                                    .into_predicates(Box::new(
                                        NeutralRequestPredicate::<ReqBody>::new(),
                                    ))?;
                            Ok(Box::new(Or::new(
                                predicate,
                                acc,
                                Box::new(NeutralRequestPredicate::new()),
                            )) as RequestPredicate<ReqBody>)
                        })
                    }
                }
            }
            Operation::And(predicates) => predicates
                .into_iter()
                .try_rfold(inner, |inner, predicate| predicate.into_predicates(inner)),
            Operation::Not(expression) => {
                let predicate =
                    expression.into_predicates(Box::new(NeutralRequestPredicate::new()))?;
                Ok(Box::new(Not::new(predicate)))
            }
        }
    }
}
