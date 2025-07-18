use std::num::NonZeroU16;

use hitbox_http::predicates::NeutralResponsePredicate;
use hitbox_http::predicates::response::StatusCode;
use hitbox_http::{
    CacheableHttpResponse,
    predicates::conditions::{Not, Or, And},
};
use serde::{Deserialize, Serialize};

type CorePredicate<ReqBody> =
    Box<dyn hitbox_core::Predicate<Subject = CacheableHttpResponse<ReqBody>> + Send + Sync>;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Predicate {
    Status(NonZeroU16),
}

impl Predicate {
    pub fn into_predicates<ReqBody: Send + 'static>(
        &self,
        inner: CorePredicate<ReqBody>,
    ) -> CorePredicate<ReqBody> {
        match self {
            Predicate::Status(code) => {
                Box::new(StatusCode::new(inner, code.get().try_into().unwrap()))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Operation {
    And(Vec<Expression>),
    Or(Vec<Expression>),
}

impl Operation {
    pub fn into_predicates<ReqBody: Send + 'static>(
        self,
        inner: CorePredicate<ReqBody>,
    ) -> CorePredicate<ReqBody> {
        match self {
            Operation::Or(predicates) => {
                let neutral = Box::new(Not::new(NeutralResponsePredicate::new()));
                Box::new(And::new(
                    inner,
                    predicates
                        .into_iter()
                        .rfold(neutral as CorePredicate<ReqBody>, |inner, predicate| {
                            let predicate =
                                predicate
                                    .into_predicates(Box::new(
                                        NeutralResponsePredicate::<ReqBody>::new(),
                                    ));
                            Box::new(Or::new(predicate, inner))
                        }),
                ))
            }
            Operation::And(predicates) => predicates
                .into_iter()
                .rfold(inner, |inner, predicate| predicate.into_predicates(inner)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Expression {
    Predicate(Predicate),
    Operation(Operation),
}

impl Expression {
    pub fn into_predicates<ReqBody: Send + 'static>(
        self,
        inner: CorePredicate<ReqBody>,
    ) -> CorePredicate<ReqBody> {
        match self {
            Self::Predicate(predicate) => predicate.into_predicates(inner),
            Self::Operation(operation) => operation.into_predicates(inner),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Response {
    Flat(Vec<Predicate>),
    Tree(Expression),
}

impl Default for Response {
    fn default() -> Self {
        Response::Flat(Vec::new())
    }
}

impl Response {
    pub fn into_predicates<Req>(self) -> CorePredicate<Req>
    where
        Req: Send + 'static,
    {
        let neutral_predicate = Box::new(NeutralResponsePredicate::<Req>::new());
        match self {
            Response::Flat(predicates) => predicates
                .iter()
                .rfold(neutral_predicate, |inner, predicate| {
                    predicate.into_predicates(inner)
                }),
            Response::Tree(expression) => expression.into_predicates(neutral_predicate),
        }
    }
}
