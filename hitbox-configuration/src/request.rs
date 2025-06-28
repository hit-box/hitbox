use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum HeaderOperation {
    Eq { name: String, value: String },
    Exist(String),
    In(String, Vec<String>),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum QueryOperation {
    Eq { name: String, value: String },
    Exist(String),
    In(String, Vec<String>),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Predicate {
    Method(String),
    Path(String),
    Query(QueryOperation),
    Header(HeaderOperation),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Expression {
    And(Box<Wrapper>, Box<Wrapper>),
    Or(Box<Wrapper>, Box<Wrapper>),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Wrapper {
    Predicate(Predicate),
    Expression(Expression),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
#[serde(untagged)]
pub enum Request {
    #[serde(rename = "requests")]
    Flat(Vec<Predicate>),
    #[serde(rename = "request")]
    Tree(Wrapper),
}
