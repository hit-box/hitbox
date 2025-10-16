use hitbox_http::predicates::{
    NeutralRequestPredicate,
    conditions::Or,
    request::{Header, Method, Path, Query},
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum HeaderOperation {
    Eq(IndexMap<String, String>),
    Exist(String),
    In(IndexMap<String, Vec<String>>),
}

impl HeaderOperation {
    fn into_predicates<ReqBody: Send + 'static>(
        &self,
        inner: RequestPredicate<ReqBody>,
    ) -> RequestPredicate<ReqBody> {
        match self {
            HeaderOperation::Eq(params) => params.iter().rfold(inner, |inner, (key, value)| {
                Box::new(Header::new(
                    inner,
                    hitbox_http::predicates::request::header::Operation::Eq(
                        key.parse().unwrap(),
                        value.parse().unwrap(),
                    ),
                ))
            }),
            HeaderOperation::Exist(param) => Box::new(Header::new(
                inner,
                hitbox_http::predicates::request::header::Operation::Exist(param.parse().unwrap()),
            )),
            HeaderOperation::In(params) => params.iter().rfold(inner, |inner, (key, values)| {
                Box::new(Header::new(
                    inner,
                    hitbox_http::predicates::request::header::Operation::In(
                        key.parse().unwrap(),
                        values.iter().map(|v| v.parse().unwrap()).collect(),
                    ),
                ))
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(tag = "operation")]
pub enum QueryOperation {
    Eq(IndexMap<String, String>),
    Exist(String),
    In(IndexMap<String, Vec<String>>),
}

impl QueryOperation {
    fn into_predicates<ReqBody: Send + 'static>(
        &self,
        inner: RequestPredicate<ReqBody>,
    ) -> RequestPredicate<ReqBody> {
        match self {
            QueryOperation::Eq(params) => params.iter().rfold(inner, |inner, (key, value)| {
                Box::new(Query::new(
                    inner,
                    hitbox_http::predicates::request::query::Operation::Eq(
                        key.clone(),
                        value.clone(),
                    ),
                ))
            }),
            QueryOperation::Exist(param) => Box::new(Query::new(
                inner,
                hitbox_http::predicates::request::query::Operation::Exist(param.to_string()),
            )),
            QueryOperation::In(params) => params.iter().rfold(inner, |inner, (key, values)| {
                Box::new(Query::new(
                    inner,
                    hitbox_http::predicates::request::query::Operation::In(
                        key.clone(),
                        values.clone(),
                    ),
                ))
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum MethodOperation {
    Eq(String),
    In(Vec<String>),
}

impl MethodOperation {
    fn into_predicates<ReqBody: Send + 'static>(
        &self,
        inner: RequestPredicate<ReqBody>,
    ) -> RequestPredicate<ReqBody> {
        match self {
            MethodOperation::Eq(method) => Box::new(Method::new(inner, method.as_str()).unwrap()),
            MethodOperation::In(methods) => {
                let methods: Vec<http::Method> = methods
                    .iter()
                    .map(|m| m.parse().unwrap())
                    .collect();
                Box::new(Method::new_in(inner, methods))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Predicate {
    Method(MethodOperation),
    Path(String),
    Query(QueryOperation),
    Header(HeaderOperation),
}

impl Predicate {
    pub fn into_predicates<ReqBody: Send + 'static>(
        &self,
        inner: RequestPredicate<ReqBody>,
    ) -> RequestPredicate<ReqBody> {
        match self {
            Predicate::Method(method_operation) => method_operation.into_predicates(inner),
            Predicate::Path(path) => Box::new(Path::new(inner, path.as_str().into())),
            Predicate::Query(query_operation) => query_operation.into_predicates(inner),
            Predicate::Header(header_operation) => header_operation.into_predicates(inner),
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
        &self,
        inner: RequestPredicate<ReqBody>,
    ) -> RequestPredicate<ReqBody> {
        match self {
            Operation::Or(predicates) => {
                if predicates.is_empty() {
                    inner
                } else {
                    predicates.iter().skip(1).fold(
                        predicates[0].into_predicates(Box::new(NeutralRequestPredicate::new())),
                        |acc, predicate| {
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
                        },
                    )
                }
            }
            Operation::And(predicates) => predicates
                .iter()
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
        &self,
        inner: RequestPredicate<ReqBody>,
    ) -> RequestPredicate<ReqBody> {
        match self {
            Self::Predicate(predicate) => predicate.into_predicates(inner),
            Self::Operation(operation) => operation.into_predicates(inner),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Request {
    Flat(Vec<Predicate>),
    Tree(Expression),
}

impl Default for Request {
    fn default() -> Self {
        Self::Flat(Vec::default())
    }
}

impl Request {
    pub fn into_predicates<Req>(self) -> RequestPredicate<Req>
    where
        Req: Send + 'static,
    {
        let neutral_predicate = Box::new(NeutralRequestPredicate::<Req>::new());
        match self {
            Request::Flat(predicates) => predicates
                .iter()
                .rfold(neutral_predicate, |inner, predicate| {
                    predicate.into_predicates(inner)
                }),
            Request::Tree(expression) => expression.into_predicates(neutral_predicate),
        }
    }

    pub fn predicates<Req>(&self) -> RequestPredicate<Req>
    where
        Req: Send + 'static,
    {
        let neutral_predicate = Box::new(NeutralRequestPredicate::<Req>::new());
        match self {
            Request::Flat(predicates) => predicates
                .iter()
                .rfold(neutral_predicate, |inner, predicate| {
                    predicate.into_predicates(inner)
                }),
            Request::Tree(expression) => expression.into_predicates(neutral_predicate),
        }
    }
}
