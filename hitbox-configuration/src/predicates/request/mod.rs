use hitbox_http::FromBytes;
use hitbox_http::predicates::NeutralRequestPredicate;
use hyper::body::Body as HttpBody;
use serde::{Deserialize, Serialize};

use crate::RequestPredicate;

mod body;
mod expression;
mod header;
mod method;
mod operation;
mod predicate;
mod query;

pub use body::BodyPredicate;
pub use expression::Expression;
pub use header::HeaderOperation;
pub use method::MethodOperation;
pub use operation::Operation;
pub use predicate::Predicate;
pub use query::QueryOperation;

// Use untagged enum - serde tries Flat (array) first, then Tree
#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
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
        Req: HttpBody + FromBytes + Send + 'static,
        Req::Error: std::fmt::Debug,
        Req::Data: Send,
    {
        let neutral_predicate = Box::new(NeutralRequestPredicate::<Req>::new());
        match self {
            Request::Flat(predicates) => predicates
                .into_iter()
                .rfold(neutral_predicate, |inner, predicate| {
                    predicate.into_predicates(inner)
                }),
            Request::Tree(expression) => expression.into_predicates(neutral_predicate),
        }
    }
}
