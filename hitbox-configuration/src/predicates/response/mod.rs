use std::num::NonZeroU16;

use hitbox_http::predicates::NeutralResponsePredicate;
use hitbox_http::predicates::response::{BodyPredicate as BodyPredicateTrait, StatusCode};
use hitbox_http::{CacheableHttpResponse, FromBytes, predicates::conditions::Or};
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

type CorePredicate<ReqBody> =
    Box<dyn hitbox_core::Predicate<Subject = CacheableHttpResponse<ReqBody>> + Send + Sync>;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum BodyPredicate {
    Jq(String),
    ProtoBuf {
        proto: String,
        message: String,
        expression: String,
    },
}

impl BodyPredicate {
    fn into_predicates<ReqBody>(
        &self,
        inner: CorePredicate<ReqBody>,
    ) -> CorePredicate<ReqBody>
    where
        ReqBody: HttpBody + FromBytes + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
        match self {
            BodyPredicate::Jq(expression) => Box::new(inner.body(
                hitbox_http::predicates::response::body::ParsingType::Jq,
                expression.clone(),
                // For Jq expressions, we expect the expression to evaluate to a boolean
                // (e.g., '.field == "value"' returns true/false)
                // We cache when the expression evaluates to true
                hitbox_http::predicates::response::body::Operation::Eq(
                    serde_json::Value::Bool(true),
                ),
            )),
            BodyPredicate::ProtoBuf {
                proto,
                message,
                expression: _,
            } => {
                // TODO: Load the MessageDescriptor from the proto file path
                // For now, this will panic if used - needs proper proto file loading
                todo!("ProtoBuf support requires loading .proto files from path: {} message: {}", proto, message)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Predicate {
    Status(NonZeroU16),
    Body(BodyPredicate),
}

impl Predicate {
    pub fn into_predicates<ReqBody>(
        &self,
        inner: CorePredicate<ReqBody>,
    ) -> CorePredicate<ReqBody>
    where
        ReqBody: HttpBody + FromBytes + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
        match self {
            Predicate::Status(code) => {
                Box::new(StatusCode::new(inner, code.get().try_into().unwrap()))
            }
            Predicate::Body(body_predicate) => body_predicate.into_predicates(inner),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Operation {
    And(Vec<Expression>),
    Or(Vec<Expression>),
}

impl Operation {
    pub fn into_predicates<ReqBody>(
        self,
        inner: CorePredicate<ReqBody>,
    ) -> CorePredicate<ReqBody>
    where
        ReqBody: HttpBody + FromBytes + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
        match self {
            Operation::Or(predicates) => {
                let mut predicates = predicates.into_iter();
                let left = predicates
                    .next()
                    .map(|expression| {
                        expression.into_predicates(
                            Box::new(NeutralResponsePredicate::new()) as CorePredicate<ReqBody>
                        )
                    })
                    .unwrap_or(Box::new(NeutralResponsePredicate::new()));
                // FIX: use Not(NeutralResponsePredicate) instead of NeutralResponsePredicate
                let right = predicates
                    .next()
                    .map(|expression| {
                        expression.into_predicates(
                            Box::new(NeutralResponsePredicate::new()) as CorePredicate<ReqBody>
                        )
                    })
                    .unwrap_or(Box::new(NeutralResponsePredicate::new()));
                let acc = Box::new(Or::new(left, right, inner));
                predicates.rfold(acc, |acc, expression| {
                    Box::new(Or::new(
                        acc,
                        expression.into_predicates(
                            Box::new(NeutralResponsePredicate::new()) as CorePredicate<ReqBody>
                        ),
                        Box::new(NeutralResponsePredicate::new()),
                    ))
                })
            }
            Operation::And(predicates) => predicates
                .into_iter()
                .fold(inner, |inner, predicate| predicate.into_predicates(inner)),
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
    pub fn into_predicates<ReqBody>(
        self,
        inner: CorePredicate<ReqBody>,
    ) -> CorePredicate<ReqBody>
    where
        ReqBody: HttpBody + FromBytes + Send + 'static,
        ReqBody::Error: std::fmt::Debug,
        ReqBody::Data: Send,
    {
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
        Req: HttpBody + FromBytes + Send + 'static,
        Req::Error: std::fmt::Debug,
        Req::Data: Send,
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
