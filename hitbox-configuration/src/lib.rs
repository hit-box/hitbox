use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum HeaderOperation {
    Eq { name: String, value: String },
    Exist(String),
    In(String, Vec<String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum QueryOperation {
    Eq { name: String, value: String },
    Exist(String),
    In(String, Vec<String>),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Predicate {
    Method(String),
    Query(QueryOperation),
    Header(HeaderOperation),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Expression {
    Predicate(Predicate),
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Request {
    #[serde(rename = "requests")]
    Flat(Vec<Predicate>),
    #[serde(rename = "request")]
    Tree(Expression),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Endpoint {
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    pub request: Request,
}
