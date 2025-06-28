use hitbox_http::{
    CacheableHttpRequest,
    predicates::{
        NeutralRequestPredicate,
        request::{Method, Query},
    },
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum HeaderOperation {
    Eq { name: String, value: String },
    Exist(String),
    In(String, Vec<String>),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
// #[serde(untagged)]
#[serde(tag = "operation")]
pub enum QueryOperation {
    Eq(IndexMap<String, String>),
    Exist(String),
    In(IndexMap<String, Vec<String>>),
    // RegExp(String, String),
}
impl QueryOperation {
    fn into_predicates<ReqBody: Send + 'static>(
        &self,
        inner: Box<
            dyn hitbox_core::Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
        >,
    ) -> Box<dyn hitbox_core::Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync>
    {
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
            QueryOperation::Exist(_) => todo!(),
            QueryOperation::In(hash_map) => todo!(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Predicate {
    Method(String),
    Path(String),
    Query(QueryOperation),
    Header(HeaderOperation),
}

impl Predicate {
    pub fn into_predicates<ReqBody: Send + 'static>(
        &self,
        inner: Box<
            dyn hitbox_core::Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync,
        >,
    ) -> Box<dyn hitbox_core::Predicate<Subject = CacheableHttpRequest<ReqBody>> + Send + Sync>
    {
        match self {
            Predicate::Method(method) => Box::new(Method::new(inner, method.as_str()).unwrap()),
            Predicate::Path(_) => todo!(),
            Predicate::Query(query_operation) => query_operation.into_predicates(inner),
            Predicate::Header(header_operation) => todo!(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Operation {
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Expression {
    Predicate(Predicate),
    Operation(Operation),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Request {
    Flat(Vec<Predicate>),
    Tree(Expression),
}

impl Request {
    pub fn into_predicates<Req>(
        &self,
    ) -> Box<dyn hitbox_core::Predicate<Subject = CacheableHttpRequest<Req>> + Send + Sync>
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
            Request::Tree(expression) => unimplemented!(),
        }
    }
}
